use nix::unistd::Pid;
use owo_colors::OwoColorize;
use rustc_hash::FxHashMap;
use std::path::{PathBuf};
use crate::{dwarf::debug_info::DebugVariable, session::{DebugSession, operations::{ManagedOperation, OperationResult, OperationTarget, PlatformOperation}, variable::DebugValue}, sys::{SystemError, linux::{PlatformWatchpoint, watchpoint::{HardwareDebugRegister, WatchAccess, WatchSize}}}, utils::trim_file_path};
use crate::session::ParamType;

/// A [`PlatformWatchpoint`] wrapped in a [`ManagedOperation`]
pub type ManagedWatchpoint = ManagedOperation<PlatformWatchpoint>;


/// Empty implementation, only for allowing [`PlatformWatchpoint`] to be used as 
/// a Platform operation in [`ManagedOperation`]
impl PlatformOperation for PlatformWatchpoint {}


#[derive(Debug)]
/// Contains all the architecture for manipulating **watchpoint** in the session. Instead of having directly two new [`FxHashMap`]s and a [`Vec`] in the [`DebugSession`],
/// the [`DebugSession`] only contains this structure.
pub struct WatchPoints {
    /// The vector of indexed [`WatchpointData`] (like for **breakpoints**)
    pub watchpoints_index_tracker: Vec<Option<WatchpointData>>,

    /// Maps the addresses to the active [`ManagedWatchpoint`]s
    pub active_watchpoints: FxHashMap<u64, ManagedWatchpoint>,
    
    /// Maps DebugRegisters <-> watchpoints addresses
    pub registers_map: FxHashMap<HardwareDebugRegister, u64>
}

impl WatchPoints {
    /// Returns an empty [`WatchPoints`] structure. Used when initializing the [`DebugSession`].
    pub fn new() -> Self {
        Self { watchpoints_index_tracker: vec![], active_watchpoints: FxHashMap::default(), registers_map: FxHashMap::default() }
    }

    /// Returns [`Some`] `mutable` [`ManagedWatchpoint`] from the active watchpoints if exists. Else, returns [`None`].
    pub fn get_mut_from_address(&mut self, addr: u64) -> Option<&mut ManagedWatchpoint> {
        self.active_watchpoints.get_mut(&addr)
    }

    /// Pushes a [`WatchpointData`] to the `watchpoints_index_tracker`.
    pub fn push_data_to_tracker(&mut self, data: WatchpointData) {
        self.watchpoints_index_tracker.push(Some(data));
    }


    /// If a register is free, returns [`Some`] [`HardwareDebugRegister`] (the available one), and updates the map to insert it as used.
    /// \
    /// Else, if no register is available, returns [`None`].
    pub fn find_free_register_and_update(&mut self, addr: u64) -> Option<HardwareDebugRegister> {
        for register in [HardwareDebugRegister::Dr0, HardwareDebugRegister::Dr1, HardwareDebugRegister::Dr2, HardwareDebugRegister::Dr3] {
            if !self.registers_map.contains_key(&register) {

                // We update the hashmap with the new register + address of the watchpoint that uses the register
                self.registers_map.insert(register, addr);

                return Some(register); 
            }
        }

        None
    }

    /// Clears a [`ManagedWatchpoint`] or update its references count. If the watchpoint uses a [`HardwareDebugRegister`], clears it too.
    pub fn delete_watchpoint(&mut self, addr: u64, pid: Pid) -> Result<bool, SystemError> {
        let mut removed = false;

        if let Some(wp) = self.active_watchpoints.get_mut(&addr) {
            if wp.ref_count > 1 {

                wp.ref_count -= 1;
            
            } else {
                wp.operation.disable(pid)?;
                
                if let Some(reg) = wp.operation.register() {
                    
                    // Free a [`HardwareDebugRegister`], removing it in the map.
                    self.registers_map.remove(reg);
                }

                removed = true;
            }
        }

        Ok(removed)
    }
}

impl DebugSession {
    /// Returns a [`WatchResult`] if the register corresponds to an active [`ManagedWatchpoint`]. Else, returns [`None`].
    /// \
    /// ### How it works
    /// 
    /// If and address corresponding to the given [`HardwareDebugRegister`] exists and if this address matches the one of a [`WatchpointData`], the method gets this [`WatchpointData`] as mutable, to manipulate and treat it.
    /// \
    /// Then, the methods retrieves the new value that the variable contains, stores the `old value` and replace the `old_value` field in the [`WatchpointData`] by the new value.
    /// \
    /// Finally, it returns a [`WatchResult`] that contains all the informations to display.
    pub fn get_changed_watchpoint_from_register(&mut self, register: &HardwareDebugRegister) -> Result<Option<WatchResult>, SystemError> {
        if let Some(addr) = self.watchpoints.registers_map.get(register) // If the hardware register correspond to an address, we retrieve it

            // And we retrieve the corresponding watchpoint data, as mutable because we'll change the `old_value`
            && let Some(wp_data) =  
                self.watchpoints.watchpoints_index_tracker.iter_mut()
                    .filter_map(|opt| opt.as_mut())  // Convert Vec<Option<T>> -> iterator of &mut T
                    .find(|wp| wp.target.address == *addr)
                    
        {   
            // We get the new value
            let new_value = match self.metadata.type_index.get(&wp_data.var.target_type_offset) {
                Some(node) => node.dwarf_type.to_debug_value(&self.metadata.type_index, wp_data.target.address, self.pid, &self.process_map)?,
                None => None
            };

            // We get the old value
            let old_value = wp_data.old_value.clone();

            // If the old value is equal to the new value and the access requested is `Changed`, we return None (and we don't notify the user)
            if old_value == new_value && matches!(wp_data.access, WatchAccess::Changed) {
                return Ok(None)
            }

            // We update the value in the `WatchpointData`
            wp_data.old_value = new_value.clone();

            return Ok(Some(WatchResult {
                address: wp_data.target.address,
                var_name: wp_data.var.name.clone(),
                line: wp_data.line,
                old_value,
                new_value,
                file: wp_data.target.file.clone()
            }))
        }

        Ok(None)
    }

    /// Given the name of the variable the user wants to put a watchpoint on, the [`WatchAccess`] he wants, and the declaration name of the variable, retrieves the corresponding [`DebugVariable`] and tries to create a [`WatchpointData`].
    /// \
    /// ### How it works
    /// 
    /// First, the function calls [`DebugSession::find_specified_param`] to retrieve the current scope.
    /// \
    /// Then, it calls [`get_param_with_name_and_line`] and if it returns [`Some`] [`DebugVariable`], it continues (else, the function returns [`Ok(OperationResult::NotFound)`])
    /// \
    /// After that, the function retrieves all the informations needed for parsing the [`DebugVariable`] and retrieving its address (it retrieves exactl the `file path`, the `encoding`, the `frame_base` and the [`WatchSize`] for the watchpoint creation).
    /// \
    /// With the `encoding`, the `frame base` and the `file path`, it parses the address of the [`DebugVariable`].
    /// \
    /// Then it retrieves its current value (using the `DwarfType` and the [`to_debug_value`] method), and creates the [`OperationTarget`].
    /// \
    /// Finally, it creates the [`WatchpointData`], calls [`DebugSession::create_specific_watchpoint`] and updates the `index tracker`.
    /// 
    /// ### Warning
    /// 
    /// If the variable isn't in the current scope, the function will fail and return [`OperationResult::NotFound`]. It is expected, but many factors can make the function fail, so a real error can remain silent.
    /// \
    /// Also, if the users try to create more than **4** watchpoints, the program will panic because of [`unimplemented!`] in [`crate::sys::linux::watchpoint::Watchpoint`]. This is also expected - for now - since software watchpoints
    /// have not yet been implemented.
    /// 
    /// ### Usage
    /// 
    /// This method is used in [`crate::cli::utils::handle_watchpoint_setting`].
    /// 
    pub fn create_watchpoint(&mut self, var_name: &str, access: WatchAccess, decl_line: u32) -> Result<OperationResult, SystemError> {
        let node: crate::dwarf::debug_info::ActiveParamsContext<'_> = self.find_specified_param(ParamType::Variable)?;  

        if let Some(var) = node.get_param_with_name_and_line(var_name, decl_line) {
            let regs = self.get_regs()?;

            let file_path = match self.current_location() {
                Some(loc) => match self.interner.get_string(loc.file) {
                    Some(str) => str,
                    None => return Ok(OperationResult::NotFound)
                },
                None => {
                    return Ok(OperationResult::NotFound)
                    
                }
            };

            let encoding = match self.metadata.encoding {
                Some(encoding) => encoding,
                None => {
                    return Ok(OperationResult::NotFound)}
            };

            let frame_base = match node.frame_base {
                Some(base) => base,
                None => return Ok(OperationResult::NotFound)
            };

            let size = match WatchSize::from_option_u64(self.get_variable_size(var)) {
                Some(size) => size,
                None => {
                    return Ok(OperationResult::NotFound)
                }
            };

            let address = match var.parse_value(&regs, encoding, self.metadata.endian, &self.metadata.abi, frame_base, &self.metadata.type_index, self.pid, &self.process_map) {
                Ok(Some(addr)) => addr,
                _ => return Ok(OperationResult::NotFound)
            };

            let value = match self.metadata.type_index.get(&var.target_type_offset) {
                Some(node) => node.dwarf_type.to_debug_value(&self.metadata.type_index, address, self.pid, &self.process_map)?,
                None => None
            };

            let target =  OperationTarget { file: Box::new(PathBuf::from(file_path)), address };


            let data = WatchpointData {
                access,
                target: target.clone(),
                line: decl_line,
                var: Box::new(var.clone()),
                enabled: true,
                size,
                old_value: value
            };

            self.create_specific_watchpoint(address, size, access)?;
            
            self.watchpoints.watchpoints_index_tracker.push(Some(data));

            Ok(OperationResult::Created { count: 1, target })
        } else {
            Ok(OperationResult::NotFound)
        }
    }

    /// Creates a specific [`PlatformWatchpoint`] and enables it.
    /// \
    /// ### How it works
    /// \
    /// The method retrieves the absolute address of the variable, verifies that a [`ManagedWatchpoint`] doesn't already exists for this address (if yes, it just updates the `ref_count`).
    /// \
    /// Then, it tries finding a free [`HardwareDebugRegister`], creates a [`PlatformWatchpoint`] and enables it.
    /// \
    /// Finally, it inserts the [`ManagedWatchpoint`] in the `active_watchpoints` map.
    pub fn create_specific_watchpoint(&mut self, relative_address: u64, size: WatchSize, access: WatchAccess) -> Result<(), SystemError> {
        let absolute_address = self.get_absolute_address(relative_address);

        // If watchpoint already exists, we dont write : simply increment the reference counter
        if let Some(managed_watchpoint) = self.watchpoints.get_mut_from_address(absolute_address) {
            managed_watchpoint.ref_count += 1;
            return Ok(());
        }

        // We try finding a free register
        let register = self.watchpoints.find_free_register_and_update(relative_address);

        // We create the watchpoint
        let mut watchpoint = PlatformWatchpoint::new(relative_address, size, access, register);

        // And enable it
        watchpoint.enable(self.pid)?;

        self.watchpoints.active_watchpoints.insert(absolute_address, ManagedWatchpoint::new(watchpoint));

        Ok(())
    }

    /// Clears a [`ManagedWatchpoint`] of the given address, by just retrieving its absolute address and calling [`WatchPoints::delete_watchpoint`].
    pub fn clear_specific_watchpoint(&mut self, relative_address: u64) -> Result<(), SystemError> {
        let absolute_address = self.get_absolute_address(relative_address);

        self.watchpoints.delete_watchpoint(absolute_address, self.pid)?;

        Ok(())
    }

    /// Enables a **watchpoint**. If the watchpoint is already enabled, returns [`OperationResult::AlreadyInState`].
    /// \
    /// Else, calls [`DebugSession::create_specific_watchpoint`], and updates the `watchpoints_index_tracker`
    pub fn enable_watchpoint(&mut self, idx: usize) -> Result<OperationResult, SystemError> {
        let target = self.watchpoints.watchpoints_index_tracker[idx].clone();

        if let Some(mut data) = target {
            if data.enabled {
                return Ok(OperationResult::AlreadyInState);
            }

            self.create_specific_watchpoint(data.target.address, data.size, data.access)?;

            data.enabled = true;

            self.watchpoints.watchpoints_index_tracker[idx] = Some(data);

            return Ok(OperationResult::Updated)
        }

        Ok(OperationResult::NotFound)

    }

    /// Disables a **watchpoint** by clearing it calling [`DebugSession::clear_specific_watchpoint`] and by udpdating `watchpoints_index_tracker`.
    /// \
    /// If the **watchpoint** is already disabled, returns [`OperationResult::AlreadyInState`].
    pub fn disable_watchpoint(&mut self, idx: usize) -> Result<OperationResult, SystemError> {
        let target = self.watchpoints.watchpoints_index_tracker[idx].clone();

        if let Some(mut data) = target {
            if !data.enabled {
                return Ok(OperationResult::AlreadyInState);
            }

            self.clear_specific_watchpoint(data.target.address)?;

            data.enabled = false;

            self.watchpoints.watchpoints_index_tracker[idx] = Some(data);
            return Ok(OperationResult::Updated);
        }

        Ok(OperationResult::NotFound)
    }

    /// Deletes a watchpoint, by removing its [`WatchpointData`] from the `watchpoints_index_tracker` and by clearing it calling
    /// [`DebugSession::clear_specific_watchpoint`]
    pub fn delete_watchpoint(&mut self, idx: usize) -> Result<OperationResult, SystemError> {
        let target = self.watchpoints.watchpoints_index_tracker[idx].take();

        if let Some(data) = target {
            self.clear_specific_watchpoint(data.target.address)?;
            return  Ok(OperationResult::Updated);
        }

        Ok(OperationResult::NotFound)
    }
}


#[derive(Debug, Clone)]
/// The **data** associated with a [`ManagedWatchpoint`]. Useful for disabling a **watchpoint** without deleting it, retrieving the value of the targeted variable...
pub struct WatchpointData {
    /// The associated [`OperationTarget`]
    pub target: OperationTarget,

    /// The declaration line of the corresponding variable
    pub line: u32,

    /// Either it is enabled or not
    pub enabled: bool,

    /// The access to watch
    pub access: WatchAccess,

    /// The size of the variable to watch
    pub size: WatchSize,

    /// The watched variable
    var: Box<DebugVariable>,

    /// Previous value of the variable
    pub old_value: Option<DebugValue>,
}

impl std::fmt::Display for WatchpointData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let enabled = if self.enabled { "enabled" } else { "disabled" };

        let target = &self.target;
        let file_path = trim_file_path(*target.file.clone());
        let value = if let Some(value) = &self.old_value {value} else {&DebugValue::None};

        write!(
            f,
            "watchpoint {} at {}:{} and address {:#x} ; targeted variable : {}, value : {}",
            enabled.red(),
            file_path.green(),
            self.line,
            target.address.bright_blue(),
            self.var.name,
            value
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
/// A structure that implements [`std::fmt::Display`], and used for displaying the result of a [`ManagedWatchpoint`] mutation.
/// \
/// Used by [`DebugSession::get_changed_watchpoint_from_register`]
pub struct WatchResult {
    pub address: u64,
    pub var_name: String,
    pub line: u32,
    pub old_value: Option<DebugValue>,
    pub new_value: Option<DebugValue>,
    pub file: Box<PathBuf>,
}

impl std::fmt::Display for WatchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let file_path = trim_file_path(*self.file.clone());

        let old_value = if let Some(value) = &self.old_value {value} else {&DebugValue::None};
        let new_value = if let Some(value) = &self.new_value {value} else {&DebugValue::None};

        write!(
            f,
            "Variable {} at {}:{} and address {:#x} changed from {} to {}",
            self.var_name.cyan(),
            file_path.green(),
            self.line,
            self.address.bright_blue(),
            old_value.cyan(),
            new_value.bright_cyan(),
        )?;

        Ok(())
    }
}
