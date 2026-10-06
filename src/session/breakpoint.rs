use crate::{session::{DebugSession, operations::{ManagedOperation, OperationTarget, PlatformOperation}}, sys::linux::PlatformBreakpoint};
use owo_colors::OwoColorize;
use std::path::Path;
use crate::sys::SystemError;
use crate::utils::trim_file_path;

/// A [`PlatformBreakpoint`] wrapped in a [`ManagedOperation`]
pub type ManagedBreakpoint = ManagedOperation<PlatformBreakpoint>;

/// Empty implementation, only for allowing [`PlatformBreakpoint`] to be used as 
/// a Platform operation in [`ManagedOperation`]
impl PlatformOperation for PlatformBreakpoint {

}

#[derive(Debug, Clone)]
pub struct BreakpointData {
    pub target: Vec<OperationTarget>,
    pub line: u32,
    pub file: Box<Path>,
    pub enabled: bool,
}

impl DebugSession {
    /// Create a specific breakpoint at a given address
    pub fn create_specific_breakpoint(&mut self, relative_address: u64) -> Result<(), SystemError> {
        let absolute_address = self.get_absolute_address(relative_address);

        // If breakpoint already exists dont write simply increment the reference counter
        if let Some(managed_breakpoint) = self.active_breakpoints.get_mut(&absolute_address) {
            managed_breakpoint.ref_count += 1;
            return Ok(());
        }

        // First time seeing the address
        // Create the breakpoint
        let mut breakpoint = crate::sys::os::PlatformBreakpoint::new(absolute_address);
        breakpoint.enable(self.pid)?;

        self.active_breakpoints
            .insert(absolute_address, ManagedBreakpoint::new(breakpoint));

        Ok(())
    }

    /// Clear breakpoint at specfic breakpoint address
    pub fn clear_specific_breakpoint(&mut self, relative_address: u64) -> Result<(), SystemError> {
        let absolute_address = self.get_absolute_address(relative_address);
        let mut should_remove = false;

        // If breakpoint doesnt exist, simply ignore it
        if let Some(managed_breakpoint) = self.active_breakpoints.get_mut(&absolute_address) {
            if managed_breakpoint.ref_count > 1 {
                // Other breakpoints exist, dont remove it, simply decrement
                managed_breakpoint.ref_count -= 1;
            } else {
                managed_breakpoint.operation.disable(self.pid)?;
                should_remove = true;
            }
        }

        if should_remove {
            self.active_breakpoints.remove(&absolute_address);
        }

        Ok(())
    }

    /// Get absolute address ( the sum of base address and absolute address )
    pub fn get_absolute_address(&self, relative_address: u64) -> u64 {
        self.base_address + relative_address
    }

    /// Get breakpoint target (file name and relative address ) from line number and file name
    pub fn get_specific_breakpoint_target(
        &self,
        file_name: &str,
        line_number: u32,
    ) -> Vec<OperationTarget> {
        let Some(line_index) = self.get_operation_target(line_number) else {
            return vec![];
        };

        line_index
            .into_iter()
            .filter(|x| x.file.ends_with(file_name))
            .collect()
    }

    /// Enable breakpoint at a specific index in the tracker
    pub fn enable_breakpoint(
        &mut self,
        index: usize,
    ) -> Result<BreakpointMutationResult, SystemError> {
        // NOTE: Safe index, bounds are checked by the cli
        let target = self.breakpoint_index_tracker[index].clone();

        if let Some(mut data) = target {
            // If already enabled, DO NOTHING
            if data.enabled {
                return Ok(BreakpointMutationResult::AlreadyInState);
            }

            for bp in data.target.iter() {
                self.create_specific_breakpoint(bp.address)?
            }

            data.enabled = true;

            // Update the actual session instance
            self.breakpoint_index_tracker[index] = Some(data);
            return Ok(BreakpointMutationResult::Updated);
        }

        Ok(BreakpointMutationResult::NotFound)
    }

    /// Disable breakpoint and returns true if successful
    pub fn disable_breakpoint(
        &mut self,
        index: usize,
    ) -> Result<BreakpointMutationResult, SystemError> {
        // NOTE: Safe index, bounds are checked by the cli
        let target = self.breakpoint_index_tracker[index].clone();

        if let Some(mut data) = target {
            // If already disabled, DO NOTHING
            if !data.enabled {
                return Ok(BreakpointMutationResult::AlreadyInState);
            }

            for bp in data.target.iter() {
                self.clear_specific_breakpoint(bp.address)?;
            }

            data.enabled = false;

            // Update the actual session instance
            self.breakpoint_index_tracker[index] = Some(data);
            return Ok(BreakpointMutationResult::Updated);
        }

        Ok(BreakpointMutationResult::NotFound)
    }

    /// Deletes breakpoint and returns true if successful
    pub fn delete_breakpoint(
        &mut self,
        index: usize,
    ) -> Result<BreakpointMutationResult, SystemError> {
        // NOTE: Safe index, bounds are checked by the cli
        let target = self.breakpoint_index_tracker[index].take();

        if let Some(data) = target {
            for bp in data.target.iter() {
                self.clear_specific_breakpoint(bp.address)?
            }
            return Ok(BreakpointMutationResult::Updated);
        }

        Ok(BreakpointMutationResult::NotFound)
    }

    /// Create breakpoint(s) at a file on a given line number
    /// Returns the number of breakpoint targets that were found on the given line alongside the address/first target if multiple addresses exist
    pub fn create_breakpoint(
        &mut self,
        line_number: u32,
        file: &Path,
    ) -> Result<BreakpointMutationResult, SystemError> {
        let Some(line_index) = self.get_operation_target(line_number) else {
            return Ok(BreakpointMutationResult::NotFound);
        };

        let line_index: Vec<OperationTarget> = line_index
            .into_iter()
            .filter(|bp| *bp.file == *file)
            .collect();

        let mut bp_for_line = 0;
        for bp in line_index.iter() {
            self.create_specific_breakpoint(bp.address)?;
            bp_for_line += 1;
        }

        self.breakpoint_index_tracker
            .push(Some(BreakpointData::from_target(
                line_index.clone(),
                line_number,
                file,
            )));

        Ok(BreakpointMutationResult::Created {
            count: bp_for_line as u8,
            target: line_index[0].clone(),
        })
    }

    /// Clear all breakpoint for line_number by default
    /// Only clear specified breakpoints if file name is provided
    pub fn clear_breakpoint(
        &mut self,
        line_number: u32,
        file: Option<&str>,
    ) -> Result<Vec<usize>, SystemError> {
        let mut cleared_breakpoints = Vec::new();
        let mut bp_idx = Vec::new();

        let filter_by_file = file.is_some();

        // Map every breakpoints that macthes the user's choice into being None
        // Store these breakpoints and their indices
        for (idx, opt_bp) in self.breakpoint_index_tracker.iter_mut().enumerate() {
            if let Some(bp) = opt_bp {
                if !filter_by_file 
                    && bp.line == line_number {
                        if let Some(removed_bp) = opt_bp.take() {
                            cleared_breakpoints.push(removed_bp);
                            bp_idx.push(idx + 1);
                        }
                } else {
                    if let Some(bp_file) = bp.file.to_str() {
                        // Safe unwrap since this is the path where the file is Some
                        if bp.line == line_number && bp_file.ends_with(file.unwrap())
                            && let Some(removed_bp) = opt_bp.take() {
                                cleared_breakpoints.push(removed_bp);
                                bp_idx.push(idx + 1);
                            }
                    } else {
                        eprintln!("[Warning] Failed to convert file path at index {}", idx)
                    }
                }
            }
        }

        for data in cleared_breakpoints.iter() {
            for bp in data.target.iter() {
                self.clear_specific_breakpoint(bp.address)?
            }
        }

        Ok(bp_idx)
    }
}

impl BreakpointData {
    pub fn from_target(target: Vec<OperationTarget>, line: u32, file: &Path) -> Self {
        Self {
            target,
            line,
            file: Box::from(file),
            enabled: true,
        }
    }
}

impl std::fmt::Display for BreakpointData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let enabled = if self.enabled { "y" } else { "n" };

        if self.target.len() == 1 {
            let target = &self.target[0];
            let file_path = trim_file_path(*target.file.clone());

            write!(
                f,
                "breakpoint\tkeep {}\t{:#x} at {}:{}",
                enabled,
                target.address.bright_blue(),
                file_path.green(),
                self.line
            )?;
        } else {
            writeln!(f, "breakpoint\tkeep {}\t<MULTIPLE>", enabled)?;

            for (idx, target) in self.target.iter().enumerate() {
                let user_idx = idx + 1;
                write!(
                    f,
                    "  .{}\t\t     {}\t{:#x} at {}:{}",
                    user_idx.cyan(),
                    enabled,
                    target.address.blue(),
                    trim_file_path(*target.file.clone()).green(),
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

#[derive(Debug)]
pub enum BreakpointMutationResult {
    Created { count: u8, target: OperationTarget },
    Updated,
    AlreadyInState,
    NotFound,
}
