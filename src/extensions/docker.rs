use async_trait::async_trait;
use eyre::Result;

use crate::KitchenConfig;
use crate::cmd::ScriptRunner;
use crate::extensions::Extension;

pub struct Docker {}

impl Docker {
    pub fn from_toml(_v: &toml::Value) -> Result<Self> {
        Ok(Self {})
    }
}

#[async_trait]
impl Extension for Docker {
    fn name(&self) -> &'static str {
        "docker"
    }

    async fn install(&self, _k: &KitchenConfig) -> Result<()> {
        ScriptRunner::from_url("https://get.docker.com")
            .await?
            .label("install docker")
            // TODO sudo?
            .run()
            .await?;

        Ok(())
    }
}
