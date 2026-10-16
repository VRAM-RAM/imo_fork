use crate::sys::macos::error::MacOSError;
use crate::sys::os::syscalls;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use mach2::structs::x86_thread_state64_t;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
/// Stores and manages the insertion and removal of the 0xCC (INT3) keyword for the instruction
#[derive(Debug)]
pub struct BreakPoint {
    addr: u64,
    original_byte: u8,
    is_enabled: bool,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl BreakPoint {
    /// Creates a new instance of breakpoint from address
    pub fn new(addr: u64) -> Self {
        Self {
            addr,
            original_byte: 0,
            is_enabled: false,
        }
    }

    /// Overwrite the lowest byte with 0xCC (INT3) while saving the original byte
    pub fn enable(&mut self, task_port: u32) -> Result<(), MacOSError> {
        // Read the current memory word
        let word = syscalls::peek_data(task_port, self.addr)?;

        // Save the original lowest byte
        self.original_byte = (word & 0xFF) as u8;

        unsafe {
            let arr: [u8; 1] = 0xCC_u8.to_le_bytes();
            let ptr = arr.as_ptr() as usize;

            let mut kern_return = mach2::vm::mach_vm_protect(
                task_port,
                self.addr,
                1,
                0,
                mach2::vm_prot::VM_PROT_READ
                    | mach2::vm_prot::VM_PROT_COPY
                    | mach2::vm_prot::VM_PROT_WRITE,
            );

            if kern_return != mach2::kern_return::KERN_SUCCESS {
                eprintln!("Could not protect address : {}", kern_return);
            }

            kern_return = mach2::vm::mach_vm_write(task_port, self.addr, ptr, 1);

            if kern_return != mach2::kern_return::KERN_SUCCESS {
                eprintln!("Could not write to address : {}", kern_return);
            }

            // Revert permissions back to default
            kern_return = mach2::vm::mach_vm_protect(
                task_port,
                self.addr,
                1,
                0,
                mach2::vm_prot::VM_PROT_EXECUTE | mach2::vm_prot::VM_PROT_READ,
            );

            if kern_return != mach2::kern_return::KERN_SUCCESS {
                eprintln!("Could not protect address : {}", kern_return);
            }
        }

        self.is_enabled = true;

        Ok(())
    }

    /// Swaps 0xCC out, puts original byte back
    pub fn disable(&mut self, task_port: u32) -> Result<(), MacOSError> {
        if !self.is_enabled {
            return Ok(());
        }

        let word = syscalls::peek_data(task_port, self.addr)?;
        let restored_word = (word & !0xFF) | (self.original_byte as i64);

        unsafe {
            mach2::vm::mach_vm_write(task_port, self.addr, restored_word as usize, 0);
        }
        self.is_enabled = false;

        Ok(())
    }
}
