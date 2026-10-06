use std::mem::offset_of;

use nix::{libc, unistd::Pid};
use rustc_hash::FxHashMap;

use crate::sys::{SystemError, linux::{ProcessId, watchpoint::HardwareDebugRegister::{Dr0, Dr1, Dr2, Dr3, Dr6, Dr7}}};

/// Stores a **watchpoint**.
#[derive(Debug, Default)]
pub struct Watchpoint {
    /// The address of the variable to watch
    addr: u64,

    /// The size of the region to watch, stored as a [`WatchSize`].
    size: WatchSize,

    /// The access the user wants to watch. See [`WatchAccess`] for more infos.
    access: WatchAccess,

    /// If the WatchPoint is configured with an Hardware register (CPU-specific register for debugging), its value is stored here.
    register: Option<HardwareDebugRegister>,

    /// Either if the [`Watchpoint`] is enabled or not. 
    /// It is unused for now (since we use hardware watchpoints), but later it will be useful :
    /// the user will have the ability to create watchpoints, and keep them but disable them.
    enabled: bool,
}

impl Watchpoint {
    pub fn enable(&mut self, pid: ProcessId) -> Result<(), SystemError> {
        //  
        if let Some(register) = &self.register {

        } else {

        }

        Ok(())
    }

    fn enable_hardware(pid: ProcessId, register: &HardwareDebugRegister, access: &WatchAccess, size: &WatchSize) -> Result<(), SystemError> {
        todo!("Implement hardware watchpoints")
    }

    fn enable_software(&mut self, pid: ProcessId) -> Result<(), SystemError> {
        todo!("Implement software watchpoints")
    }
}
/// The size of the buffer to watch
/// (an enum and not an [`u8`], because the size is limited by the hardware)
/// \
/// When we'll implement the watchpoints using SoftWare, we'll change this enum.
#[derive(Debug, Default, Clone)]
pub enum WatchSize {
    #[default]
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
}

#[derive(Debug, Default)]
/// A CPU's **debug** register. The four debug registers are `DR0`, `DR1`, `DR2` and `DR3`. 
/// \
/// `DR6` and `DR7` are also used by the watchpoints, but aren't debug registers.
pub enum HardwareDebugRegister {
    #[default]
    Dr0,
    Dr1,
    Dr2,
    Dr3,

    Dr6,
    Dr7,
}

impl HardwareDebugRegister {
    /// Tries converting an [`usize`] into a [`HardwareDebugRegister`].
    /// \
    /// Returns [`Some`] [`HardwareDebugRegister`] if the conversion is possible, else returns [`None`].
    pub fn from_usize(value: usize) -> Option<Self> {
        match value.into() {
            0 => Some(Dr0),
            1 => Some(Dr1),
            2 => Some(Dr2),
            3 => Some(Dr3),
            6 => Some(Dr6),
            7 => Some(Dr7),
            _ => None
        }
    }

    /// You can already try converting an [`usize`] into an [`HardwareDebugRegister`] using [`HardwareDebugRegister::from_usize`], so this 
    /// implementation allows you to convert an [`HardwareDebugRegister`] into an [`usize`], which is useful for writing the register, for example.
    pub fn to_usize(&self) -> usize {
        match self {
            Self::Dr0 => 0,
            Self::Dr1 => 1,
            Self::Dr2 => 2,
            Self::Dr3 => 3,
            Self::Dr6 => 6,
            Self::Dr7 => 7,
        }
    }
    
    /// Writes an [`u64`] to the corresponding register.
    /// \
    /// Example :
    /// ```rust
    /// use imo::sys::linux::watchpoint::HardwareDebugRegister;
    /// use nix::unistd::Pid;
    /// 
    /// fn foo(pid: Pid, addr: u64) {
    ///     let register = HardwareDebugRegister::Dr0;
    ///     register.write(pid, addr).unwrap();
    /// 
    ///     HardwareDebugRegister::Dr7.write(pid, 1u64).unwrap();
    /// }
    /// ```
    pub fn write(&self, pid: Pid, value: u64) -> nix::Result<()> {
        // `ptrace` exposes `libc::user`, that contains 'u_debugreg' (an array that contains the 8 debug registers).
        // 
        // Here, we compute the offset, in bytes, of the start of 'u_debugreg' in  `libc::user`.
        // Then, `self.to_usize()` * `std::mem::size_of::<libc::c_ulong>()` computes the offset for the wanted register.
        //
        // For example, if c_ulong is an u64 (most of the cases)
        // Dr0 : base (u_debugreg) + 0
        // Dr1 : base (u_debugreg) + 8 
        // ...
        // Dr7 : base (u_debugreg) + 56
        let offset = offset_of!(libc::user, u_debugreg) + self.to_usize() * std::mem::size_of::<libc::c_ulong>();

        // Calls `ptrace` with the POKEUSER operation.
        // This operations copies the word data to offset addr in the tracee's USER area.
        // (more informations : https://man7.org/linux/man-pages/man2/ptrace.2.html)
        // 
        // So here, we write the `value` in the given debug register of the process of given Pid
        let ret = unsafe {
            libc::ptrace(libc::PTRACE_POKEUSER, pid.as_raw(), offset, value as libc::c_ulong)
        };

        // ptrace returns an int.
        // If its value is -1, an error occured, and `nix::Error::last()` catches this error.
        if ret == -1 {
            return Err(nix::Error::last())
        }
        Ok(())
    }
}


/// The access 'kind' the user wants.
/// - Read : catch it, and notify the user if the variable is read
/// - Write : catch it, and notify the user if the variable is written
/// - ReadWrite : catch it, and notify the user if the variable is written or read.
#[derive(Debug, Default)]
pub enum WatchAccess {
    Read,
    Write,

    #[default]
    ReadWrite,
}
