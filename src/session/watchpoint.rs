use owo_colors::OwoColorize;
use std::path::Path;

use crate::utils::trim_file_path;

#[derive(Debug, Clone)]
pub struct WatchpointTarget {
    pub file: Box<Path>,
    pub relative_address: u64,
}

#[derive(Debug, Clone)]
pub struct WatchpointData {
    pub target: Vec<WatchpointTarget>,
    pub line: u32,
    pub file: Box<Path>,
    pub enabled: bool,
}

impl WatchpointData {
    pub fn from_target(target: Vec<WatchpointTarget>, line: u32, file: &Path) -> Self {
        Self {
            target,
            line,
            file: Box::from(file),
            enabled: true,
        }
    }
}

impl std::fmt::Display for WatchpointData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let enabled = if self.enabled { "y" } else { "n" };

        if self.target.len() == 1 {
            let target = &self.target[0];
            let file_path = trim_file_path(&target.file);

            write!(
                f,
                "watchpoint\tkeep {}\t{:#x} at {}:{}",
                enabled,
                target.relative_address.bright_blue(),
                file_path.green(),
                self.line
            )?;
        } else {
            writeln!(f, "watchpoint\tkeep {}\t<MULTIPLE>", enabled)?;

            for (idx, target) in self.target.iter().enumerate() {
                let user_idx = idx + 1;
                write!(
                    f,
                    "  .{}\t\t     {}\t{:#x} at {}:{}",
                    user_idx.cyan(),
                    enabled,
                    target.relative_address.blue(),
                    trim_file_path(&target.file).green(),
                    self.line
                )?;

                // Avoid an extra trailing newline at the very last location
                if idx < self.target.len() - 1 {
                    writeln!(f)?;
                }
            }
        }
        Ok(())
    }
}