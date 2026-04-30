use async_trait::async_trait;
use std::collections::HashSet;

use crate::config::KitchenToml;
use crate::image::ContextFile;
use crate::kitchen::KitchenConfig;


pub mod dotfiles;
pub mod pitchfork;
pub mod tailscale;


#[async_trait]
pub trait Extension: Send + Sync {
    fn name(&self) -> &'static str;

    fn image_context(&self, _k: &KitchenConfig) -> Result<Vec<ContextFile>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }

    fn install(&self, _k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn onstart(&self, _k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn poststart(&self, _k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

type Builder = fn(&toml::Value) -> Result<Box<dyn Extension>, Box<dyn std::error::Error>>;

const REGISTRY: &[(&str, Builder)] = &[
    ("dotfiles", |v| Ok(Box::new(dotfiles::Dotfiles::from_toml(v)?))),
    ("pitchfork", |v| Ok(Box::new(pitchfork::Pitchfork::from_toml(v)?)))
];

pub fn build(toml: Option<&KitchenToml>) -> Result<Vec<Box<dyn Extension>>, Box<dyn std::error::Error>> {
    let known: HashSet<&str> = REGISTRY.iter().map(|(n, _)| *n).collect();
    let empty_table = toml::Table::new();
    let configs = toml.map(|t| &t.extension_configs).unwrap_or(&empty_table);

    // Typo Protection: make sure everything in extension_configs is a
    // registered extension
    for key in configs.keys() {
        if !known.contains(key.as_str()) {
            return Err(format!("unknown config section: [{key}]").into())
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


pub async fn onstart(k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running kitchen onstart hooks...");

    for ext in &k.extensions { ext.onstart(k).await?; }
    Ok(())
}

pub async fn poststart(k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running kitchen poststart hooks...");

    for ext in &k.extensions { ext.onstart(k).await?; }
    tailscale::poststart(k)?;

    Ok(())
}
