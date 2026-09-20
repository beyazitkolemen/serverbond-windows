use crate::{model::tool_package, Manager};
use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

pub const ID: &str = "node";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeState {
    pub version: String,
    pub installed: bool,
    pub directory: Option<PathBuf>,
}

impl Manager {
    pub(crate) fn node_state(&self) -> NodeState {
        let package = tool_package(ID).expect("embedded node package");
        let directory = self.tool_directory(ID).ok();
        NodeState {
            version: package.version,
            installed: directory.is_some(),
            directory,
        }
    }

    /// Node is not a service: it only joins the project terminal's PATH.
    pub(crate) fn tool_directory(&self, id: &str) -> Result<PathBuf> {
        let executable = self.tool_executable(id)?;
        Ok(executable
            .parent()
            .expect("tool executables live in their package directory")
            .to_path_buf())
    }

    pub fn install_node(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.install_tool(ID)?;
        self.log("Node.js kuruldu. Proje terminalinde node, npm ve npx kullanılabilir.");
        Ok(())
    }

    pub fn repair_node(&self) -> Result<()> {
        let _guard = self.gate()?;
        self.repair_tool(ID)
    }
}
