/*
Log Adapters Module

This module contains infrastructure adapters that implement the LogPort trait
for different logging backends and purposes.
*/

pub mod system_log_adapter;
// pub mod error_log_adapter;
pub mod access_log_adapter;

// Re-export the main adapters for easier importing
pub use system_log_adapter::{SystemLogAdapter, SystemLogAdapterConfig};
// pub use error_log_adapter::{ErrorLogAdapter, ErrorLogAdapterConfig, ErrorInfo};