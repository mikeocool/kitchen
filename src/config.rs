use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct KitchenToml {
    pub name: Option<String>,

    pub dotfiles_repo: Option<String>,
    pub dotfiles_install_cmd: Option<String>,

    pub features: Option<Features>,
    pub container: Option<Container>,
}

#[derive(Deserialize, Debug)]
pub struct Container {
    pub workspace_mount_path: Option<String>,
    // TODO support multiple networks
    pub network: Option<String>,
    pub additional_mounts: Option<Vec<Mount>>,
}

#[derive(Deserialize, Debug)]
pub struct Mount {
    pub source: String,
    pub target: String,
    #[serde(rename = "type", default = "MountType::default")]
    pub mount_type: MountType,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum MountType {
    #[default]
    Bind,
    Volume,
    Tmpfs,
    Image,
    Npipe,
    Cluster,
}

#[derive(Deserialize, Debug)]
pub struct Features {
    pub docker_outside_of_docker: Option<bool>,
}

pub fn load(workspace: &PathBuf) -> Result<Option<KitchenToml>, Box<dyn std::error::Error>> {
    let path = workspace.join(".kitchen.toml");
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(&path)?;
    Ok(Some(toml::from_str(&contents)?))
}
