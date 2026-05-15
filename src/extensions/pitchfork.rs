use async_trait::async_trait;
use eyre::{Result, eyre};
use std::io::Write;
use std::process::{Command, Stdio};

use crate::cmd::ScriptRunner;
use crate::extensions::Extension;
use crate::image::Containerfile;
use crate::kitchen::KitchenConfig;

const INSTALL_SCRIPT: &str = include_str!("../../resources/pitchfork/install.sh");
const ONSTART_SCRIPT: &str = include_str!("../../resources/pitchfork/onstart.sh");

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

    fn image_instructions(&self, _k: &KitchenConfig) -> Result<Option<Containerfile>> {
        Ok(Some(Containerfile::new().run(
            "mkdir -p /etc/pitchfork/ && chown k:k /etc/pitchfork/",
        )))
    }

    async fn install(&self, _k: &KitchenConfig) -> Result<()> {
        // doesnt need sudo because install runs as root
        ScriptRunner::script(INSTALL_SCRIPT)
            .label("Install pitchfork")
            .run()
            .await?;

        Ok(())
    }

    async fn onstart(&self, _k: &KitchenConfig) -> Result<()> {
        ScriptRunner::script(ONSTART_SCRIPT)
            .label("Setup pitchfork config")
            .sudo()
            .run()
            .await?;

        Ok(())
    }
}
