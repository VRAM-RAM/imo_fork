use crate::cli::handle_user_debugger_menu;
use crate::error::DebuggerError;
use crate::session;
use crate::sys::{macos::error::MacOSError, os::syscalls};

use mach2::{kern_return, mach_types, traps};
use rustyline::DefaultEditor;

#[cfg(target_arch = "x86_64")]
use mach2::thread_status::x86_THREAD_STATE64 as THREAD_STATE64;
#[cfg(target_arch = "aarch64")]
use mach2::thread_status::ARM_THREAD_STATE64 as THREAD_STATE64;

/// Struct matching the message format of the message being received
/// from EXCEPTION_STATE_IDENTITY

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionStateIdentityMessage {
    pub header: mach2::message::mach_msg_header_t,
    pub body: mach2::message::mach_msg_body_t,
    pub padding: u32, // Match the alignment of the message

    pub thread: mach2::message::mach_msg_port_descriptor_t,
    pub task: mach2::message::mach_msg_port_descriptor_t,

    pub exception: u32,
    pub code_count: u32,
    pub code: [i32; 2],

    pub flavor: i32,
    pub state_count: u32,
    pub thread_state: crate::sys::os::PlatformRegStruct,
}

pub fn debug(rl: &mut DefaultEditor, binary_path: &str) -> Result<(), DebuggerError> {
    let mut pid: i32 = 0;
    let path = std::ffi::CString::new(binary_path).map_err(|e| MacOSError::CString(e))?;

    let mut spawn_attr: libc::posix_spawnattr_t = std::ptr::null_mut();
    let return_value = unsafe { libc::posix_spawnattr_init(&mut spawn_attr) };

    if return_value != 0 {
        eprintln!("Error creating attr");
        return Ok(());
    }

    let flag = libc::POSIX_SPAWN_START_SUSPENDED as i16;
    let return_value = unsafe { libc::posix_spawnattr_setflags(&mut spawn_attr, flag) };

    if return_value != 0 {
        eprintln!("Error creating flag");
        return Ok(());
    }

    let return_value = unsafe {
        libc::posix_spawn(
            &mut pid,
            path.as_ptr(),
            std::ptr::null(),
            &mut spawn_attr,
            std::ptr::null(),
            std::ptr::null(),
        )
    };

    if return_value != 0 {
        eprintln!("Error creating process");
        return Ok(());
    }

    let mut child_task_port: mach2::port::mach_port_name_t = 0;

    let mut kern_return =
        unsafe { traps::task_for_pid(traps::mach_task_self(), pid, &mut child_task_port) };

    println!("Port: {} and Kern return: {}", child_task_port, kern_return);

    unsafe {
        println!("{}", libc::getpid());
    }

    if kern_return == kern_return::KERN_SUCCESS {
        let mut session = session::DebugSession::new(child_task_port, binary_path)?;
        let mut exception_port: mach2::port::mach_port_name_t = 0;

        unsafe {
            let mut child_name: mach2::port::mach_port_name_t = 0;

            // Allocate the exception port
            kern_return = mach2::mach_port::mach_port_allocate(
                traps::mach_task_self(),
                mach2::port::MACH_PORT_RIGHT_RECEIVE,
                &mut exception_port,
            );

            if kern_return != kern_return::KERN_SUCCESS {
                eprintln!("Failed to create exception port")
            }

            kern_return = mach2::mach_port::mach_port_insert_right(
                traps::mach_task_self(),
                exception_port,
                exception_port,
                mach2::message::MACH_MSG_TYPE_MAKE_SEND,
            );

            if kern_return != kern_return::KERN_SUCCESS {
                eprintln!("Failed to insert exception port")
            }

            // set specific exceptions to be tracked
            kern_return = mach2::task::task_set_exception_ports(
                child_task_port,
                mach2::exception_types::EXC_MASK_BREAKPOINT,
                exception_port,
                mach2::exception_types::EXCEPTION_STATE_IDENTITY as i32,
                THREAD_STATE64,
            );

            if kern_return != kern_return::KERN_SUCCESS {
                println!("kern return: {}", kern_return);
                eprintln!("failed to set exception port")
            }

            let mut prev = 0;
            // include tracking for when the task finishes
            kern_return = mach2::mach_port::mach_port_request_notification(
                traps::mach_task_self(),
                child_task_port,
                mach2::notify::MACH_NOTIFY_DEAD_NAME,
                0,
                exception_port,
                mach2::message::MACH_MSG_TYPE_MAKE_SEND_ONCE,
                &mut prev,
            );

            if kern_return != kern_return::KERN_SUCCESS {
                println!("KERN RETURN: {}", kern_return);
                eprintln!("Failed to create destroyed exception");
            }

            kern_return = mach2::task::task_suspend(child_task_port);

            if kern_return != kern_return::KERN_SUCCESS {
                println!("Could not suspend child");
            }

            let res = libc::ptrace(libc::PT_ATTACHEXC, pid, std::ptr::null_mut(), 0);

            if res != 0 {
                eprintln!("failed to attach to process");
            }
        }

        handle_user_debugger_menu(&mut session, rl)?;

        let mut msg_header = mach2::message::mach_msg_header_t::default();
        let option = mach2::message::MACH_RCV_MSG;
        let size = 0;
        let recieve = unsafe { traps::mach_task_self() };
        let timeout = mach2::message::MACH_MSG_TIMEOUT_NONE;
        let notify = mach2::port::MACH_PORT_NULL;

        loop {
            kern_return = unsafe {
                mach2::message::mach_msg(
                    &mut msg_header,
                    option,
                    size,
                    recieve,
                    exception_port,
                    timeout,
                    notify,
                )
            };

            if kern_return != kern_return::KERN_SUCCESS {
                eprintln!("Error retrieving message");
            } else {
            }
        }
    }
    Ok(())
}
