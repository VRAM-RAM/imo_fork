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

        self.original_byte = (word & 0xFF) as u8;
        let arr: [u8; 1] = 0xCC_u8.to_le_bytes();
        syscalls::write_data(task_port, self.addr, &arr)?;

        self.is_enabled = true;

        Ok(())
    }

    /// Swaps 0xCC out, puts original byte back
    pub fn disable(&mut self, task_port: u32) -> Result<(), MacOSError> {
        if !self.is_enabled {
            return Ok(());
        }

        let restored_word: [u8; 1] = self.original_byte.to_le_bytes();
        syscalls::write_data(task_port, self.addr, &restored_word);
        self.is_enabled = false;

        Ok(())
    }
}
