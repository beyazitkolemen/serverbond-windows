//! Action enums parsed from IPC/CLI strings: tool lifecycle, GitHub token
//! handling and environment start/stop. Wire strings never change.

use anyhow::{bail, Result};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolAction {
    Install,
    Repair,
    Start,
    Stop,
    Open,
}

impl ToolAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Repair => "repair",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Open => "open",
        }
    }
}

impl FromStr for ToolAction {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "install" => Ok(Self::Install),
            "repair" => Ok(Self::Repair),
            "start" => Ok(Self::Start),
            "stop" => Ok(Self::Stop),
            "open" => Ok(Self::Open),
            _ => bail!("Bilinmeyen hizmet işlemi: {value}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GithubAction {
    Save,
    Forget,
    Import,
}

impl GithubAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Save => "save",
            Self::Forget => "forget",
            Self::Import => "import",
        }
    }
}

impl FromStr for GithubAction {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "save" => Ok(Self::Save),
            "forget" => Ok(Self::Forget),
            "import" => Ok(Self::Import),
            _ => bail!("Bilinmeyen GitHub işlemi: {value}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvironmentAction {
    Start,
    Stop,
}

impl EnvironmentAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
        }
    }
}

impl FromStr for EnvironmentAction {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "start" => Ok(Self::Start),
            "stop" => Ok(Self::Stop),
            _ => bail!("Bilinmeyen ortam işlemi: {value}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_actions_round_trip() {
        for action in [
            ToolAction::Install,
            ToolAction::Repair,
            ToolAction::Start,
            ToolAction::Stop,
            ToolAction::Open,
        ] {
            assert_eq!(action.as_str().parse::<ToolAction>().unwrap(), action);
        }
        assert!("wipe".parse::<ToolAction>().is_err());
    }

    #[test]
    fn github_actions_parse() {
        for action in [
            GithubAction::Save,
            GithubAction::Forget,
            GithubAction::Import,
        ] {
            assert_eq!(action.as_str().parse::<GithubAction>().unwrap(), action);
        }
        assert!("token".parse::<GithubAction>().is_err());
    }

    #[test]
    fn environment_actions_round_trip() {
        assert_eq!(
            EnvironmentAction::Start
                .as_str()
                .parse::<EnvironmentAction>()
                .unwrap(),
            EnvironmentAction::Start
        );
        assert_eq!(
            "stop".parse::<EnvironmentAction>().unwrap(),
            EnvironmentAction::Stop
        );
        assert!("restart".parse::<EnvironmentAction>().is_err());
    }
}
