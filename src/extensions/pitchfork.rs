use async_trait::async_trait;
use eyre::{Result, eyre};
use std::io::Write;
use std::process::{Command, Stdio};

use crate::extensions::Extension;
use crate::kitchen::KitchenConfig;

const SCRIPT: &str = include_str!("../../resources/pitchfork/onstart.sh");

pub struct Pitchfork {}

impl Pitchfork {
    pub fn from_toml(_v: &toml::Value) -> Result<Self> {
        Ok(Self {})
    }
}

#[async_trait]
impl Extension for Pitchfork {
    fn name(&self) -> &'static str {
        "pitchfork"
    }

    async fn onstart(&self, _k: &KitchenConfig) -> Result<()> {
        let mut child = Command::new("sudo")
            .args(["sh", "-s"])
            .stdin(Stdio::piped())
            .spawn()?;

        child.stdin.as_mut().unwrap().write_all(SCRIPT.as_bytes())?;

        let status = child.wait()?;
        if !status.success() {
            return Err(eyre!("pitchfork onstart failed"));
        }

        Ok(())
    }
}
