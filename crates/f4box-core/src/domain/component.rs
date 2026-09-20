use anyhow::{bail, Result};
use std::fmt;
use std::str::FromStr;

/// Installed binary identity. These strings are process map keys and `bin/<id>/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComponentId {
    Php,
    Mysql,
    Caddy,
    Composer,
    PhpMyAdmin,
    Mailpit,
    Redis,
    Postgres,
    Cloudflared,
    Node,
}

impl ComponentId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Php => "php",
            Self::Mysql => "mysql",
            Self::Caddy => "caddy",
            Self::Composer => "composer",
            Self::PhpMyAdmin => "phpmyadmin",
            Self::Mailpit => "mailpit",
            Self::Redis => "redis",
            Self::Postgres => "postgres",
            Self::Cloudflared => "cloudflared",
            Self::Node => "node",
        }
    }
}

impl fmt::Display for ComponentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ComponentId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "php" => Ok(Self::Php),
            "mysql" => Ok(Self::Mysql),
            "caddy" => Ok(Self::Caddy),
            "composer" => Ok(Self::Composer),
            "phpmyadmin" => Ok(Self::PhpMyAdmin),
            "mailpit" => Ok(Self::Mailpit),
            "redis" => Ok(Self::Redis),
            "postgres" => Ok(Self::Postgres),
            "cloudflared" => Ok(Self::Cloudflared),
            "node" => Ok(Self::Node),
            _ => bail!("Bilinmeyen bileşen: {value}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_ids_round_trip() {
        for id in [
            ComponentId::Php,
            ComponentId::Mysql,
            ComponentId::Caddy,
            ComponentId::Composer,
            ComponentId::PhpMyAdmin,
            ComponentId::Mailpit,
            ComponentId::Redis,
            ComponentId::Postgres,
            ComponentId::Cloudflared,
            ComponentId::Node,
        ] {
            assert_eq!(id.as_str().parse::<ComponentId>().unwrap(), id);
        }
        assert!("unknown".parse::<ComponentId>().is_err());
    }
}
