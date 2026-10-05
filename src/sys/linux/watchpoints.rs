
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

/// The size of the buffer to watch
/// (an enum and not an [`u8`], because the size is limited by the hardware)
#[derive(Debug)]
pub enum WatchSize {
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
}

/// A CPU's **debug** register. The four debug registers are `DR0`, `DR1`, `DR2` and `DR3`. 
/// \
/// `DR6` and `DR7` are also used by the watchpoints, but aren't debug registers.
pub enum HardwareDebugRegister {
    Dr0,
    Dr1,
    Dr2,
    Dr3,
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

pub struct WatchPoints {
    watchpoints: Vec<Watchpoint>,
}

impl WatchPoints {
    pub fn new() -> Self {
        Self { watchpoints: vec![] }
    }

    pub fn set_watchpoint(&mut self, var: DebugVariable) {

    }
}
