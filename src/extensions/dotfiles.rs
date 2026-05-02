use async_trait::async_trait;
use eyre::{Result, eyre};
use serde::Deserialize;
use std::io::Write;
use std::process::{Command, Stdio};

use super::Extension;
use crate::kitchen::KitchenConfig;

const SCRIPT: &str = include_str!("../../resources/provision/dotfiles.sh");

pub struct Dotfiles {
    pub repo: Option<String>,
    pub install_cmd: Option<String>,
}

#[derive(Deserialize, Default)]
struct Toml {
    repo: Option<String>,
    install_cmd: Option<String>,
}

impl Dotfiles {
    pub fn from_toml(v: &toml::Value) -> Result<Self> {
        let cfg: Toml = v.clone().try_into()?;
        Ok(Self {
            repo: cfg.repo,
            install_cmd: cfg.install_cmd,
        })
    }
}

#[async_trait]
impl Extension for Dotfiles {
    fn name(&self) -> &'static str {
        "dotfiles"
    }

    async fn onstart(&self, _k: &KitchenConfig) -> Result<()> {
        let Some(repo) = &self.repo else {
            return Ok(());
        };
        let install_cmd = self.install_cmd.as_deref().unwrap_or("");

        let mut child = Command::new("sh")
            .args(["-s", "--", repo, install_cmd])
            .stdin(Stdio::piped())
            .spawn()?;

        child.stdin.as_mut().unwrap().write_all(SCRIPT.as_bytes())?;

        let status = child.wait()?;
        if !status.success() {
            return Err(eyre!("dotfiles provisioning failed"));
        }
        Ok(())
    }
}
