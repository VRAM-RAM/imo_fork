use nix::{unistd::Pid};
use bit_field::BitField;
use crate::{sys::{SystemError, linux::{ProcessId}}};

pub mod size;
pub mod debug_register;
pub mod access;

pub use self::{size::WatchSize, debug_register::HardwareDebugRegister, access::WatchAccess};

/// Stores a **watchpoint**.
#[derive(Debug)]
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

/// Returns a [`Vec`] of accessed [`HardwareDebugRegister`]s, by reading `Dr6`.
pub fn watch(pid: Pid) -> nix::Result<Vec<HardwareDebugRegister>> {
    let mut accessed_registers = Vec::with_capacity(4);

    let dr6_value = HardwareDebugRegister::Dr6.read(pid)?;

    for idx_of_wp in 0..=3 {
        let accessed = dr6_value.get_bit(idx_of_wp);
        if accessed {
            // We unwrap it, since the value is in [0; 3] (it should never panic)
            accessed_registers.push(HardwareDebugRegister::from_usize(idx_of_wp).expect("This should never panic. Please open an issue."))
        }
    }

    Ok(accessed_registers)
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
    /// For more informations, read : https://web.archive.org/web/20080730014804/http://www.codeproject.com/KB/debug/hardwarebreakpoint.aspx,
    /// and https://pdos.csail.mit.edu/6.828/2004/readings/i386/s12_02.htm#fig12-1
    fn configure_and_enable_hardware_watchpoint(pid: ProcessId, access: &WatchAccess, debug_register: &HardwareDebugRegister, size: &WatchSize) -> nix::Result<()> {
        // We first read the current value of `DR7`
        let mut dr7_value = HardwareDebugRegister::Dr7.read(pid)?;

        // Then, we compute an offset (configuring Dr0 doesn't require to write the same bits as configuring Dr3, for example)
        // Each register configuration flag takes 2 bits, that's why we multiply by 2.
        let mut offset = debug_register.to_usize() * 2;

        // At offset..=offset + 1, (so on two bits), we write 1 and 0.
        // The first bit is for enabling local watchpoint, and the second one is for
        // enabling global watchpoint.
        dr7_value.set_bits(offset..=offset + 1, 0b10);

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
    /// and https://pdos.csail.mit.edu/6.828/2004/readings/i386/s12_02.htm#fig12-1
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

    pub fn register(&self) -> &Option<HardwareDebugRegister> {
        &self.register
    }
}



