/// The access 'kind' the user wants.
/// - Changed : the variable was writtent AND changed (for example, if it is written but the value is the same, it returns nothing)
/// - Write : catch it, and notify the user if the variable is written
/// - ReadWrite : catch it, and notify the user if the variable is written or read.
#[derive(Debug, Default, Copy, Clone)]
pub enum WatchAccess {
    Changed,

    Write,

    #[default]
    ReadWrite,
}

impl WatchAccess {
    /// Converts the [`WatchAccess`] into a flag for `DR7` [`HardwareDebugRegister`]
    pub fn to_bits_flag(&self) -> u64 {
        match self {
            Self::ReadWrite => 0b11,
            Self::Write => 0b01,
            Self::Changed => 0b01
        }
    }

    /// Converts a [`str`] into a [`WatchAccess`]. If the value doesn't correspond to any [`WatchAccess`], returns [`None`]
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "readwrite" | "rw" | "rewr" => Some(Self::ReadWrite),
            "write" | "w" | "wr" => Some(Self::Write),
            "changed" | "c" | "onchange" => Some(Self::Changed),
            _ => None
        }
    }
}