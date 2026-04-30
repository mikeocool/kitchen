use std::path::PathBuf;

use bollard::models::{Mount, MountTypeEnum, MountBindOptions};

use crate::config;
use crate::extensions;
use crate::extensions::Extension;

pub struct KitchenConfig {
    pub name: String,
    pub local_workspace_path: PathBuf,
    pub container_workspace_path: String,
    pub container: ContainerConfig,
    pub extensions: Vec<Box<dyn Extension>>,
}

impl KitchenConfig {
    pub fn from_workspace(
        workspace: &Option<PathBuf>,
    ) -> Result<KitchenConfig, Box<dyn std::error::Error>> {
        let local_workspace_path = match workspace {
            Some(ws) => std::fs::canonicalize(ws).unwrap_or_else(|_| ws.clone()),
            None => std::env::current_dir().expect("failed to get current directory"),
        };

        let config_toml = config::load(&local_workspace_path)?;
        let config_toml = config_toml.as_ref();

        let workspace_dir_name = local_workspace_path
            .file_name()
            .expect("workspace path has no final component") // TODO return error that name needs to be specified in config or with arg
            .to_string_lossy()
            .into_owned();

        let name = config_toml
            .and_then(|c| c.name.clone())
            .unwrap_or(workspace_dir_name);

        let container_workspace_path = format!("/workspaces/{}", name);

        let container = ContainerConfig::from_config(
            config_toml.and_then(|c| c.container.as_ref()),
            local_workspace_path.as_path(),
        );

        let extensions = extensions::build(config_toml)?;

        Ok(KitchenConfig {
            name,
            local_workspace_path,
            container_workspace_path,
            container,
            extensions,
        })
    }

    pub fn container_name(&self) -> String {
        format!("{}-kitchen", self.name)
    }

    pub fn kitchen_workspace_env(&self) -> String {
        format!("KITCHEN_WORKSPACE={}", self.container_workspace_path)
    }
}

pub struct ContainerConfig {
    pub host_workspace_path: String,
    pub additional_mounts: Vec<Mount>,
    pub network: Option<String>,
}

impl ContainerConfig {
    pub fn from_config(
        config_toml: Option<&config::Container>,
        local_workspace_path: &std::path::Path,
    ) -> Self {
        // TODO this wrong is we're running in the container
        let host_workspace_path = config_toml
            .and_then(|c| c.workspace_mount_path.as_deref())
            .unwrap_or_else(|| local_workspace_path.to_str().unwrap_or_default())
            .to_string();

        let additional_mounts_toml = config_toml
            .and_then(|c| c.additional_mounts.as_deref() )
            .unwrap_or_default();

        let mut mounts = Vec::new();
        for mount_toml in additional_mounts_toml {
            mounts.push(Mount {
                typ: Some(MountTypeEnum::BIND),
                source: Some(mount_toml.source.clone()),
                target: Some(mount_toml.target.clone()),
                bind_options: Some(MountBindOptions {
                    create_mountpoint: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }

        Self {
            host_workspace_path,
            additional_mounts: mounts,
            network: config_toml.and_then(|c| c.network.clone()),
        }
    }
}
