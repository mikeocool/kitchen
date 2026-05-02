use async_trait::async_trait;
use std::collections::HashSet;

use eyre::{Result, eyre};

use crate::config::KitchenToml;
use crate::image::ContextFile;
use crate::kitchen::KitchenConfig;

pub mod dotfiles;
pub mod pitchfork;
pub mod tailscale;

#[async_trait]
pub trait Extension: Send + Sync {
    fn name(&self) -> &'static str;

    fn image_context(&self, _k: &KitchenConfig) -> Result<Vec<ContextFile>> {
        Ok(vec![])
    }
    // TODO container_config -- add container config that gets merged into existing config

    async fn install(&self, _k: &KitchenConfig) -> Result<()> {
        Ok(())
    }

    async fn onstart(&self, _k: &KitchenConfig) -> Result<()> {
        Ok(())
    }

    async fn poststart(&self, _k: &KitchenConfig) -> Result<()> {
        Ok(())
    }
}

type Builder = fn(&toml::Value) -> Result<Box<dyn Extension>>;

const REGISTRY: &[(&str, Builder)] = &[
    ("dotfiles", |v| {
        Ok(Box::new(dotfiles::Dotfiles::from_toml(v)?))
    }),
    ("pitchfork", |v| {
        Ok(Box::new(pitchfork::Pitchfork::from_toml(v)?))
    }),
    ("tailscale", |v| {
        Ok(Box::new(tailscale::Tailscale::from_toml(v)?))
    }),
];

pub fn build(toml: Option<&KitchenToml>) -> Result<Vec<Box<dyn Extension>>> {
    let known: HashSet<&str> = REGISTRY.iter().map(|(n, _)| *n).collect();
    let empty_table = toml::Table::new();
    let configs = toml.map(|t| &t.extension_configs).unwrap_or(&empty_table);

    // Typo Protection: make sure everything in extension_configs is a
    // registered extension
    for key in configs.keys() {
        if !known.contains(key.as_str()) {
            return Err(eyre!("unknown config section: [{key}]"));
        }
    }

    let empty_value = toml::Value::Table(toml::Table::new());
    let mut out = Vec::with_capacity(REGISTRY.len());
    for (name, builder) in REGISTRY {
        let cfg = configs.get(*name).unwrap_or(&empty_value);
        out.push(builder(cfg)?);
    }
    Ok(out)
}

pub async fn install(k: &KitchenConfig) -> Result<()> {
    println!("Running kitchen install hooks...");

    for ext in &k.extensions {
        ext.install(k).await?;
    }
    Ok(())
}

pub async fn onstart(k: &KitchenConfig) -> Result<()> {
    println!("Running kitchen onstart hooks...");

    for ext in &k.extensions {
        ext.onstart(k).await?;
    }
    Ok(())
}

pub async fn poststart(k: &KitchenConfig) -> Result<()> {
    println!("Running kitchen poststart hooks...");

    for ext in &k.extensions {
        ext.poststart(k).await?;
    }
    Ok(())
}
