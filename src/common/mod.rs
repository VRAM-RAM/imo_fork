//! Common **structures** and **types** for `imo`.

mod source_code;
mod string_interning;

pub use string_interning::{StringId, StringInterner};
pub use source_code::{FileIndices, UniqueFileId, SourceCodeCache, SourceCodeDisplay, SourceLocation, SourceCodeInfo, LineRow};
