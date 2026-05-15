use async_trait::async_trait;

use eyre::Result;

use crate::cmd::ScriptRunner;
use crate::extensions::Extension;
use crate::image::ContextFile;
use crate::image::Containerfile;
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

    fn image_instructions(&self, _k: &KitchenConfig) -> Result<Option<Containerfile>> {
        Ok(Some(Containerfile::new().copy("tailscale/pitchfork.toml", "/etc/kitchen/daemons/tailscale.toml")))
    }

    async fn install(&self, _k: &KitchenConfig) -> Result<()> {
        ScriptRunner::from_url("https://tailscale.com/install.sh")
            .await?
            .label("install tailscale")
            .run()
            .await?;

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
