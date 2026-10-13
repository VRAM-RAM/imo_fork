/// The size of the buffer to watch
/// (an enum and not an [`u8`], because the size is limited by the hardware)
/// \
/// When we'll implement the watchpoints using SoftWare, we'll change this enum.
#[derive(Debug, Default, Clone, Copy)]
pub enum WatchSize {
    #[default]
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
}

impl WatchSize {
    /// Converts the [`WatchSize`] into a flag for `DR7` [`crate::sys::linux::watchpoint::HardwareDebugRegister`]
    pub fn to_bits_flag(&self) -> u64 {
        match self {
            Self::One => 0b00,
            Self::Two => 0b01,
            Self::Four => 0b11,
            Self::Eight => 0b10
        }
    }

    pub fn from_option_u64(value: Option<u64>) -> Option<Self> {
        match value {
            Some(1) => Some(Self::One),
            Some(2) => Some(Self::Two),
            Some(4) => Some(Self::Four),
            Some(8) => Some(Self::Eight),
            _ => None
        }
    }
}