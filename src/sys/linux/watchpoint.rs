use std::mem::offset_of;

use nix::{libc, unistd::Pid};
use bit_field::BitField;
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
    pub fn new(addr: u64, size: WatchSize, access: WatchAccess, register: Option<HardwareDebugRegister>) -> Self {
        Self { addr, size, access, register, enabled: false }
    }

    /// Enables a [`Watchpoint`].
    pub fn enable(&mut self, pid: ProcessId) -> Result<(), SystemError> {
        // If the watchpoint was given a debug register, its because this register is free (unused) 
        if let Some(register) = &self.register {
            
            // Writes the address of the variable in the Debug register
            register.write(pid, self.addr)?;

            // Configure and enable watchpoint
            Self::configure_and_enable_hardware_watchpoint(pid, &self.access, register, &self.size)?;

            self.enabled = true;

        } else {
            unimplemented!("SoftWare watchpoints aren't implemented yet !")
        }

        Ok(())
    }

    /// Disables a [`Watchpoint`].
    pub fn disable(&mut self, pid: ProcessId) -> Result<(), SystemError> {
        if !self.enabled {
            return Ok(());
        }

        // If the watchpoint was given a debug register, its because this register is free (unused) 
        if let Some(register) = &self.register {

            // Configure and enable watchpoint
            Self::disable_hardware_watchpoint(pid, register)?;

            self.enabled = false;

        } else {
            unimplemented!("SoftWare watchpoints aren't implemented yet !")
        }

        Ok(())
    }
    
    
    /// Enables a [`Watchpoint`] which uses the hardware, by writing `DR7`.
    /// \
    /// For more informations, read : https://web.archive.org/web/20080730014804/http://www.codeproject.com/KB/debug/hardwarebreakpoint.aspx
    fn configure_and_enable_hardware_watchpoint(pid: ProcessId, access: &WatchAccess, debug_register: &HardwareDebugRegister, size: &WatchSize) -> nix::Result<()> {
        // We first read the current value of `DR7`
        let mut dr7_value = HardwareDebugRegister::Dr7.read(pid)?;

        // Then, we compute an offset (configuring Dr0 doesn't require to write the same bits as configuring Dr3, for example)
        // Each register configuration flag takes 2 bits, that's why we multiply by 2.
        let mut offset = debug_register.to_usize() * 2;

        // At offset..=offset + 1, (so on two bits), we write 1 and 0.
        // The first bit is for enabling local watchpoint, and the second one is for
        // enabling global watchpoint.
        dr7_value.set_bits(offset..=offset + 1, 0x10);

        // We grow the offset for next step
        offset += 16;

        // We write the access flag
        dr7_value.set_bits(offset..=offset + 1, access.to_bits_flag());
        
        // We grow the offset for next step
        offset += 8;

        // We write the size flag
        dr7_value.set_bits(offset..=offset + 1, size.to_bits_flag());

        
        HardwareDebugRegister::Dr7.write(pid, dr7_value)?;

        Ok(())
    }

    /// Disables a [`Watchpoint`] which uses the hardware (a debug register), by writing `DR7`.
    /// \
    /// For more informations, read : https://web.archive.org/web/20080730014804/http://www.codeproject.com/KB/debug/hardwarebreakpoint.aspx
    fn disable_hardware_watchpoint(pid: ProcessId, debug_register: &HardwareDebugRegister) -> nix::Result<()> {
        // We first read the current value of `DR7`
        let mut dr7_value = HardwareDebugRegister::Dr7.read(pid)?;

        // Then, we compute an offset (configuring Dr0 doesn't require to write the same bits as configuring Dr3, for example)
        // Each register configuration flag takes 2 bits, that's why we multiply by 2.
        let offset = debug_register.to_usize() * 2;

        // 00 means : no local watchpoint nor global watchpoint, so no watchpoint at all
        dr7_value.set_bits(offset..=offset + 1, 0x00);

        // We don't modify anything else, the watchpoint is disabled yet.

        // And we write the new `DR7` value.
        HardwareDebugRegister::Dr7.write(pid, dr7_value)?;
        Ok(())
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

impl WatchSize {
    /// Converts the [`WatchSize`] into a flag for `DR7` [`HardwareDebugRegister`]
    pub fn to_bits_flag(&self) -> u64 {
        match self {
            Self::One => 0x00,
            Self::Two => 0x01,
            Self::Four => 0x11,
            Self::Eight => 0x10
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
/// A CPU's **debug** register. The four debug registers are `DR0`, `DR1`, `DR2` and `DR3`. 
/// \
/// `DR6` and `DR7` are also used by the watchpoints, but :
/// - DR6 is watched (it communicates if one watchpoint changed)
/// - DR7 is used for configuring the registers `DR0`, `DR1`, `DR2` and `DR3`
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

    /// Writes an the given register, and returns its value, as an u64.
    /// \
    /// Example :
    /// ```rust
    /// use imo::sys::linux::watchpoint::HardwareDebugRegister;
    /// use nix::unistd::Pid;
    /// 
    /// fn foo(pid: Pid) {
    ///     let register = HardwareDebugRegister::Dr0;
    ///     let dr0_value = register.read(pid).unwrap();
    /// 
    ///     let dr7_value = HardwareDebugRegister::Dr7.read(pid).unwrap();
    /// }
    /// ```
    pub fn read(&self, pid: Pid) -> nix::Result<u64> {
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

        // Calls `ptrace` with the PEEKUSER operation.
        // 
        // This operation reads the bytes at the offset address, and returns the bytes as a `c_long`.
        // For more informations : https://man7.org/linux/man-pages/man2/ptrace.2.html
        let ret = unsafe {
            libc::ptrace(libc::PTRACE_PEEKUSER, pid.as_raw(), offset)
        };

        if ret == -1 {
            return Err(nix::Error::last());
        }

        Ok(ret as u64)
    }
}


/// The access 'kind' the user wants.
/// - Read : catch it, and notify the user if the variable is read
/// - Write : catch it, and notify the user if the variable is written
/// - ReadWrite : catch it, and notify the user if the variable is written or read.
#[derive(Debug, Default)]
pub enum WatchAccess {
    Write,

    #[default]
    ReadWrite,
}

impl WatchAccess {
    /// Converts the [`WatchAccess`] into a flag for `DR7` [`HardwareDebugRegister`]
    pub fn to_bits_flag(&self) -> u64 {
        match self {
            Self::ReadWrite => 0x11,
            Self::Write => 0x01
        }
    }
}