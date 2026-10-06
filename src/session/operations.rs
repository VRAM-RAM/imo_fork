//! Helpers and common structures for every operations, such as **breakpoints** or **watchpoints**.

use std::path::PathBuf;

use crate::session::{DebugSession};

/// Empty trait, only for keeping linearity in the architecture (so that a `Foo` structure can't be used, by default, in a [`ManagedOperation<T>`])
/// \
/// Later, it may contain some methods.
pub trait PlatformOperation {}

#[derive(Debug, Clone)]
/// A structure used for **breakpoints** and **watchpoints**, created during [`setup_session_cache`].
pub struct OperationTarget {
    pub file: Box<PathBuf>,
    pub address: u64,
}

impl DebugSession {
    /// Get the [`OperationTarget`] (file name and relative_address) from the just line number
    pub fn get_operation_target(&self, line_number: u32) -> Option<Vec<OperationTarget>> {
        let line_index = self.line_index.get(&line_number);
        line_index.cloned()
    }
}

#[derive(Debug)]
/// A managed operations. Contains a [`PlatformOperation`] (a **PlatformBreakpoint** or a **PlatformWatchpoint**, for example), reference counted.
pub struct ManagedOperation<T: PlatformOperation> {
    pub operation: T,
    pub ref_count: usize,
}

impl<T: PlatformOperation> ManagedOperation<T> {
    /// Returns a [`ManagedOperation`] from a [`PlatformOperation`]
    pub fn new(operation: T) -> Self {
        Self { operation, ref_count: 1 }
    }
}

#[derive(Debug)]
/// An enum for the results of any [`PlatformOperation`], such as a **platform watchpoint** or a **platform breakpoint**.
pub enum OperationResult {
    Created { count: u8, target: OperationTarget },
    Updated,
    AlreadyInState,
    NotFound,
}