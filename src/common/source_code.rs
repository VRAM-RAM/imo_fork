use super::StringId;
use std::path::{Path, PathBuf};
use crate::utils::display_source_code;
use owo_colors::OwoColorize;
use std::io::{BufRead, BufReader};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Copy)]
/// Represents a location in source code.
/// \
/// Example : 
/// \
/// ```code
/// pub fn get_current_list_entry(&mut self) -> Option<Vec<SourceCodeDisplay>> {
/// 
///     // SourceLocation { file: StringId(0), line: 42 } for example
///     let start_loc = self.current_loc.copied(); 
///     
///     // Gets the String corresponding to `StringId(0)`
///     let path_str = self.interner.get_str(start_loc.file).unwrap();
///     let path = Path::new(path_str);
/// 
///     // 42 
///     let start_line_num = start_loc.line;  
///     
///    // Show lines 37-50 
///    for current_line in 37..=50 {
///        self.get_source_file(path, current_line);  
///    }
/// }
/// ```
pub struct SourceLocation {
    /// Interned Id of the file path
    pub file: StringId,

    // Line number (1-indexed)
    pub line: u32,
}

#[derive(Debug, Default)]
/// Cache entry that stores a full source file content, as a [`String`], and the line offsets (bytes positions where each line starts in the string).
pub struct SourceCodeInfo {
    /// An entire source file content
    source_code: String,

    /// Bytes positions where each line starts in the [`String`]
    line_offsets: Vec<usize>,
}


impl SourceCodeInfo {
    /// Returns the number of lines that the file contains.
    pub fn line_count(&self) -> usize {
        self.line_offsets.len()
    }
}

/// Represents the **state** of source code availability when trying to display it.
pub enum SourceCodeDisplay {
    /// The source code is fully resolved : we can display the actual content of the source file.
    FullyResolved {
        source_code: Box<str>,
        line_number: u32,
    },

    /// The source code is partially resolved : we now the [`Path`] of the file but the file isn't cached yet
    PartiallyResolved {
        path: Box<Path>,
        line_number: u32,
    },

    /// The [`StringId`] exists, but the interner data is corrupted.
    CacheCorrupt {
        id: StringId,
    },

    /// No information is available.
    Unresolved,
}

impl std::fmt::Display for SourceCodeDisplay {
    /// Displays the given [`SourceCodeDisplay`].
    /// \
    /// \
    /// If the source code is totally resolved, it is displayed by [`display_source_code`].
    /// \
    /// Else, if it is partially resolved, a message containing the [`Path`] and the line number is displayed.
    /// \
    /// Else, if the source code couldn't been resolved, an error message is printed.
    /// \
    /// Finally, if the cache is corrupted, another error message is displayed. 
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FullyResolved {
                source_code,
                line_number,
            } => {
                write!(f, "{}\t", line_number.cyan())?;
                display_source_code(f, source_code)?;
            }
            Self::PartiallyResolved { path, line_number } => write!(
                f,
                "Could not locate {}. Line number: {}.",
                path.display().bright_blue(),
                line_number.cyan()
            )?,
            Self::Unresolved => write!(f, "Could not resolve current location")?,
            Self::CacheCorrupt { id } => write!(
                f,
                "Could not resolve string with id {:?}. Restarting session might fix this issue",
                id
            )?,
        }
        Ok(())
    }
}


#[derive(Default, Debug)]
/// Holds all source file entries during the debugging session. Each entry source code is stored in a [`SourceCodeInfo`].
pub struct SourceCodeCache {
    pub entries: FxHashMap<PathBuf, SourceCodeInfo>,
}

impl SourceCodeCache {
    /// Returns the exact source code text for a given file and a given line.
    pub fn get_line_entry(&self, path: &Path, line: u32) -> Option<&str> {
        if let Some(entry) = self.entries.get(path) {
            let max_idx = entry.line_count() - 1;

            // Clamp index and make sure it never goes past the max line entries available
            let index = max_idx.min(line.saturating_sub(1) as usize);
            let line_offset = entry.line_offsets.get(index)?;

            // If the current index already approached max then we cannot get the next offset
            // In this case simply go to the end of the source code
            if index >= max_idx {
                return Some(&entry.source_code[*line_offset..]);
            }

            // Get next offset after ensuring it is not past the end of the file
            let next_line_offset = entry.line_offsets.get(index + 1)?;
            return Some(&entry.source_code[*line_offset..*next_line_offset]);
        }

        None
    }

    /// Returns the number of line of a given file. 
    pub fn get_entry_line_count(&self, path: &Path) -> Option<usize> {
        self.entries.get(path).map(|e| e.line_count())
    }

    /// Creates and insert a [`SourceCodeInfo`] for the given file, and returns the exact source code for the given line.
    pub fn create_and_get_line_entry(&mut self, path: &Path, line: u32) -> Option<&str> {
        if !path.exists() {
            return None;
        }

        let file = std::fs::File::open(path).ok()?;
        let reader = BufReader::new(file);

        let mut source_code: String = String::new();
        let mut line_offsets: Vec<usize> = Vec::new();

        for line in reader.lines() {
            line_offsets.push(source_code.len());
            source_code.push_str(line.ok()?.as_ref());
        }

        let source_code_info = SourceCodeInfo {
            source_code,
            line_offsets,
        };

        self.entries.insert(path.into(), source_code_info);
        self.get_line_entry(path, line)
    }
}

/// Acts as a **key** to look up file paths (their [`StringId`]) using [`FileIndices`].
#[derive(Default, Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct UniqueFileId {
    /// Offset within the DWARF debug info
    pub offset: usize,

    /// File index from the DWARF line prog
    pub file_idx: u64,
}

/// Stores the path (as a [`StringId`]) of each file being debugged.
pub type FileIndices = FxHashMap<UniqueFileId, StringId>;

#[derive(Debug)]
/// Maps instruction addresses to their corresponding source code location.
pub struct LineRow {
    /// The source code location (file + line number)
    pub location: SourceLocation,

    /// Start of this code block's address range
    pub start_address: u64,

    /// End of that range
    pub end_address: u64,

    /// Wether it represents a statement 
    pub is_stmt: bool,
}
