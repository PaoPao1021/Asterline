pub mod adapter;
pub mod app;
pub mod contract;
pub mod domain;
mod fs_safety;
pub mod project_state;
pub mod router;
pub mod runtime;
pub mod store;
pub mod tui;

mod managed_update;
#[cfg(windows)]
mod update;
