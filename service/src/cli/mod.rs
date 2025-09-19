pub mod framework_tool;
pub mod framework_tool_parser;

// Back-compat re-exports for existing imports: crate::cli::{FrameworkTool, resolve_or_install}
pub use framework_tool::{
    FrameworkTool,
    resolve_or_install,
};


