use async_trait::async_trait;
use eyre::Result;

use super::Extension;
use crate::cmd::ScriptRunner;
use crate::kitchen::KitchenConfig;

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

    async fn install(&self, _k: &KitchenConfig) -> Result<()> {
        ScriptRunner::from_url("https://mise.run")
            .await?
            .label("install mise")
            // TODO sudo?
            .run()
            .await?;

        Ok(())
    }
}
