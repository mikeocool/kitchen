use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct KitchenToml {
    name: Option<String>,
    container: Option<Container>,
}

#[derive(Deserialize, Debug)]
pub struct Container {
    workspace_mount_path: Option<String>,
    // TODO support multiple networks
    network: Option<String>,
}

pub fn load(workspace: &PathBuf) -> Result<Option<KitchenToml>, Box<dyn std::error::Error>> {
    let path = workspace.join(".kitchen.toml");
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(&path)?;
    Ok(Some(toml::from_str(&contents)?))
}
