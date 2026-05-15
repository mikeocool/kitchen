use async_trait::async_trait;
use eyre::Result;

use super::Extension;
use crate::cmd::ScriptRunner;
use crate::image::Containerfile;
use crate::kitchen::KitchenConfig;

const ONSTART_SCRIPT: &str = include_str!("../../resources/mise/onstart.sh");

pub struct Mise {}

impl Mise {
    pub fn from_toml(_v: &toml::Value) -> Result<Self> {
        Ok(Self {})
    }
}

#[async_trait]
impl Extension for Mise {
    fn name(&self) -> &'static str {
        "mise"
    }

    fn image_instructions(&self, _k: &KitchenConfig) -> Result<Option<Containerfile>> {
        Ok(Some(
            Containerfile::new().env("MISE_INSTALL_PATH", "/usr/local/bin/mise"),
        ))
    }

    async fn install(&self, _k: &KitchenConfig) -> Result<()> {
        ScriptRunner::from_url("https://mise.run")
            .await?
            .label("install mise")
            // TODO sudo?
            .run()
            .await?;

        Ok(())
    }

    async fn onstart(&self, _k: &KitchenConfig) -> Result<()> {
        println!(
            "KITCHEN_WORKSPACE: {}",
            std::env::var("KITCHEN_WORKSPACE").unwrap_or_default()
        );
        ScriptRunner::script(ONSTART_SCRIPT)
            .label("Setting up mise")
            .run()
            .await?;

        Ok(())
    }
}
