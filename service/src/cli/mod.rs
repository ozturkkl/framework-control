pub mod framework_tool;
pub mod framework_tool_parser;

#[cfg(target_os = "windows")]
pub mod ryzen_adj;

pub mod ryzen_adj_parser;

#[cfg(target_os = "linux")]
pub mod linux_power;

// Back-compat re-export for existing imports: crate::cli::FrameworkTool
pub use framework_tool::{
    FrameworkTool,
};

// RyzenAdj exports (Windows only)
#[cfg(target_os = "windows")]
pub use ryzen_adj::{
    RyzenAdj,
};

// Linux power exports
#[cfg(target_os = "linux")]
pub use linux_power::{
    LinuxPower,
};
