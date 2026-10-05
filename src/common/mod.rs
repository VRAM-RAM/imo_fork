//! Common **structures** and **types** for `imo`.

mod source_code;
mod string_interning;
mod current_stop_command;

pub use string_interning::{StringId, StringInterner};
pub use source_code::{FileIndices, UniqueFileId, SourceCodeCache, SourceCodeDisplay, SourceLocation, SourceCodeInfo, LineRow};
pub use current_stop_command::CurrentStopCmd;
