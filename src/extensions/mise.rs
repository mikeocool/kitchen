use eyre::Result;

use super::Extension;
use crate::kitchen::KitchenConfig;

pub struct Mise {}

impl Mise {
    pub fn from_toml(v: &toml::Value) -> Result<Self> {
        Ok(Self {})
    }
}

impl Extension for Mise {
    fn name(&self) -> &'static str {
        "mise"
    }

    async fn install(&self, _k: &KitchenConfig) -> Result {
        Ok(())
    }
}
