pub mod breakpoint;
pub mod error;
pub mod syscalls;

#[cfg(target_arch = "aarch64")]
pub use mach2::thread_status::ARM_THREAD_STATE64 as THREAD_STATE64;
#[cfg(target_arch = "x86_64")]
pub use mach2::thread_status::x86_THREAD_STATE64 as THREAD_STATE64;

#[cfg(target_arch = "x86_64")]
pub const TRAP_FLAG: u64 = 1u64 << 8;

#[cfg(target_arch = "aarch64")]
pub const SS_FLAG: u32 = 1u32 << 21;

#[cfg(target_arch = "aarch64")]
use mach2::structs::arm_thread_state64_t as thread_state64_t;

#[cfg(target_arch = "aarch64")]
pub const ARM_DEBUG_STATE64: u32 = 15;

#[cfg(target_arch = "aarch64")]
pub const ARM_DEBUG_STATE64_COUNT: u32 = 130;

#[cfg(target_arch = "aarch64")]
#[repr(C)]
#[derive(Default)]
/// `ARM_DEBUG_STATE64` struct, used for enabling single-step.
///
/// - https://github.com/apple/darwin-xnu/blob/main/osfmk/mach/arm/_structs.h#L591
/// - https://github.com/apple/darwin-xnu/blob/main/osfmk/arm64/status.c
pub struct ArmDebugState64 {
    /// Hardware breakpoints (unused), since imo uses [`BRK`](`crate::sys::macos::breakpoint::BRK`) instruction.
    pub bvr: [u64; 16],
    /// Hardware breakpoints controll (unused), since imo uses [`BRK`](`crate::sys::macos::breakpoint::BRK`) instruction
    pub bcr: [u64; 16],
    /// Hardware watchpoints addresses (to be used)
    pub wvr: [u64; 16],
    /// Hardware watchpoints controll
    pub wcr: [u64; 16],
    /// Enables or disables single step
    pub mdscr_el1: u64,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use mach2::structs::x86_thread_state64_t as thread_state64_t;

pub type ProcessHandle = u32; // task_port on mac
pub type PlatformBreakpoint = breakpoint::BreakPoint;
pub type PlatformRegStruct = thread_state64_t;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MessageReply {
    pub header: mach2::message::mach_msg_header_t,
    pub ndr: mach2::ndr::NDR_record_t,
    pub kern_return: mach2::kern_return::kern_return_t,
}
