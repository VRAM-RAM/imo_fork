use owo_colors::OwoColorize;
use rustc_hash::FxHashMap;
use std::{path::Path};
use crate::{session::{DebugSession, operations::{ManagedOperation, OperationResult, OperationTarget, PlatformOperation}}, sys::{SystemError, linux::{PlatformWatchpoint, watchpoint::{HardwareDebugRegister, WatchSize, Watchpoint}}}, utils::trim_file_path};
use crate::session::ParamType;

/// A [`PlatformWatchpoint`] wrapped in a [`ManagedOperation`]
pub type ManagedWatchpoint = ManagedOperation<PlatformWatchpoint>;


/// Empty implementation, only for allowing [`PlatformWatchpoint`] to be used as 
/// a Platform operation in [`ManagedOperation`]
impl PlatformOperation for PlatformWatchpoint {}


#[derive(Debug)]
pub struct WatchPoints {
    pub watchpoints_index_tracker: Vec<Option<WatchpointData>>,
    pub active_watchpoints: FxHashMap<u64, ManagedWatchpoint>
}

impl WatchPoints {
    pub fn new() -> Self {
        Self { watchpoints_index_tracker: vec![], active_watchpoints: FxHashMap::default() }
    }

    pub fn get_mut_from_address(&mut self, addr: u64) -> Option<&mut ManagedWatchpoint> {
        self.active_watchpoints.get_mut(&addr)
    }

    pub fn push_data_to_tracker(&mut self, data: WatchpointData) {
        self.watchpoints_index_tracker.push(Some(data));
    }
}

impl DebugSession {
    pub fn create_watchpoint(&mut self, var_name: &str, file: &Path, size: WatchSize) -> Result<OperationResult, SystemError> {
        // Find the current scope
        let node = self.find_specified_param(ParamType::Variable)?;  
        
        if let Some(param) = node.get_param_with_name(var_name) {
            let regs = self.get_regs()?;

            let encoding = match self.metadata.encoding {
                Some(encoding) => encoding,
                None => return Ok(OperationResult::NotFound)
            };

            let frame_base = match node.frame_base {
                Some(base) => base,
                None => return Ok(OperationResult::NotFound)
            };

            let address = match param.parse_value(&regs, encoding, self.metadata.endian, &self.metadata.abi, frame_base, &self.metadata.type_index, self.pid, &self.process_map) {
                Ok(Some(addr)) => addr,
                _ => return Ok(OperationResult::NotFound) 
            };
            
            self.create_specific_watchpoint(address, size)?;
            
            Ok(OperationResult::Created { count: 1, target: OperationTarget { file: Box::new(file.to_path_buf()), address } })
        } else {
            Ok(OperationResult::NotFound)
        }
    }

    pub fn create_specific_watchpoint(&mut self, relative_address: u64, size: WatchSize) -> Result<(), SystemError> {
        let absolute_address = self.get_absolute_address(relative_address);

        // If watchpoint already exists, we dont write : simply increment the reference counter
        if let Some(managed_watchpoint) = self.watchpoints.get_mut_from_address(absolute_address) {
            managed_watchpoint.ref_count += 1;
            return Ok(());
        }

        todo!();

        Ok(())

    }
}


#[derive(Debug, Clone)]
pub struct WatchpointData {
    pub target: OperationTarget,
    pub line: u32,
    pub file: Box<Path>,
    pub enabled: bool,
    pub size: WatchSize,
}

impl WatchpointData {
    pub fn from_target(target: OperationTarget, line: u32, file: &Path, size: WatchSize) -> Self {
        Self {
            target,
            line,
            file: Box::from(file),
            enabled: true,
            size
        }
    }
}

impl std::fmt::Display for WatchpointData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let enabled = if self.enabled { "y" } else { "n" };

        let target = &self.target;
        let file_path = trim_file_path(*target.file.clone());

        write!(
            f,
            "watchpoint\tkeep {}\t{:#x} at {}:{}",
            enabled,
            target.address.bright_blue(),
            file_path.green(),
            self.line
        )?;

        Ok(())
    }
}
