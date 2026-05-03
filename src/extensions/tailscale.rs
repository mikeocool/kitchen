use async_trait::async_trait;

use eyre::Result;

use crate::cmd::ScriptRunner;
use crate::extensions::Extension;
use crate::image::ContextFile;
use crate::kitchen::KitchenConfig;

const PITCHFORK_TOML: &[u8] = include_bytes!("../../resources/tailscale/pitchfork.toml");

pub struct Tailscale {}

impl Tailscale {
    pub fn from_toml(_v: &toml::Value) -> Result<Self> {
        Ok(Self {})
    }
}

#[async_trait]
impl Extension for Tailscale {
    fn name(&self) -> &'static str {
        "tailscale"
    }

    fn image_context(&self, _k: &KitchenConfig) -> Result<Vec<ContextFile>> {
        Ok(vec![ContextFile::new(
            "tailscale/pitchfork.toml",
            PITCHFORK_TOML,
        )])
    }

    async fn install(&self, _k: &KitchenConfig) -> Result<()> {
        // get and run tailscale install script, if it's not already installed
        // TODO put daemon in place
        Ok(())
    }

    async fn poststart(&self, _k: &KitchenConfig) -> Result<()> {
        ScriptRunner::command("tailscale", ["up", "--ssh"])
            .label("running tailscale up")
            .sudo()
            .run()
            .await?;

        Ok(())
    }
}
