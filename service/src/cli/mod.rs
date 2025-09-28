pub mod framework_tool;
pub mod framework_tool_parser;
pub mod ryzen_adj;
pub mod ryzen_adj_parser;

// Back-compat re-exports for existing imports: crate::cli::{FrameworkTool, resolve_or_install}
pub use framework_tool::{
    FrameworkTool,
    resolve_or_install,
};

// RyzenAdj exports
pub use ryzen_adj::{
    RyzenAdj,
};


