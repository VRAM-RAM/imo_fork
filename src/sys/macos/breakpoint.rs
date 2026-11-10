use crate::sys::macos::error::MacOSError;
use crate::sys::os::syscalls;

/// The `BRK #<imm16>` instruction.
///
/// Where `BRK` is the instruction and `imm16` is an u16 identifier.
///
/// For `imo`, we don't care about `imm`'s value, so we put it as `0`.
///
/// The exact binary of `BRK #0` is :
///
/// ```
/// 1101 0100 0010 0000 0000 0000 0000 0000
/// ```
///
/// For more informations, please refeer to :
///
/// - https://student.cs.uwaterloo.ca/~cs452/docs/rpi4b/ISA_A64_xml_v88A-2021-12_OPT.pdf
/// - https://support.arm.com/documentation/111108/2026-09/Base-Instructions/BRK--Breakpoint-instruction-?lang=en   
pub const BRK: u32 = 0xD4200000;

/// Stores and manages the insertion and removal of the 0xCC (INT3) keyword for the instruction
#[derive(Debug)]
pub struct BreakPoint {
    addr: u64,

    #[cfg(target_arch = "x86_64")]
    original_instruction: u8,

    #[cfg(target_arch = "aarch64")]
    original_instruction: u32,

    is_enabled: bool,
}

impl BreakPoint {
    /// Creates a new instance of breakpoint from address
    pub fn new(addr: u64) -> Self {
        Self {
            addr,
            original_instruction: 0,
            is_enabled: false,
        }
    }

    /// Enable a [`BreakPoint`].
    ///
    /// - On `x86_64` : Overwrite the lowest byte with 0xCC (INT3) while saving the original byte
    /// - On `arm64` : Overwrite the instruction with [`BRK`] while saving the original instruction
    pub fn enable(&mut self, task_port: u32) -> Result<(), MacOSError> {
        if self.is_enabled {
            return Ok(());
        }

        // Read the current memory word
        let word = syscalls::peek_data(task_port, self.addr)?;

        #[cfg(target_arch = "x86_64")]
        {
            self.original_instruction = (word & 0xFF) as u8;
            let arr: [u8; 1] = 0xCC_u8.to_le_bytes();
            syscalls::write_data(task_port, self.addr, &arr)?;
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.original_instruction = (word & 0xFFFFFFFF) as u32;
            let arr: [u8; 4] = BRK.to_le_bytes();
            syscalls::write_data(task_port, self.addr, &arr)?;
        }

        self.is_enabled = true;

        Ok(())
    }

    /// Disable a [`BreakPoint`].
    ///
    /// - On `x86_64` : Swaps `INT3` out, and puts the original byte back
    /// - On `arm64` : Swaps `BRK` out, and replace it by the original instruction
    pub fn disable(&mut self, task_port: u32) -> Result<(), MacOSError> {
        if !self.is_enabled {
            return Ok(());
        }

        #[cfg(target_arch = "x86_64")]
        {
            let restored_word: [u8; 1] = self.original_instruction.to_le_bytes();
            syscalls::write_data(task_port, self.addr, &restored_word)?;
        }

        #[cfg(target_arch = "aarch64")]
        {
            let restored_word: [u8; 4] = self.original_instruction.to_le_bytes();
            syscalls::write_data(task_port, self.addr, &restored_word)?;
        }

        self.is_enabled = false;

        Ok(())
    }
}
