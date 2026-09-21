//! Domain vocabulary shared by the core, the desktop commands and the CLI:
//! component identifiers and the action enums that replace loose strings.

mod action;
mod component;

pub use action::{EnvironmentAction, GithubAction, ToolAction};
pub use component::ComponentId;
