use std::path::{Path, PathBuf};

use bollard::models::{Mount, MountBindOptions, MountTypeEnum};
use eyre::Result;

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
    pub fn from_workspace(workspace: &Option<PathBuf>) -> Result<KitchenConfig> {
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
        )?;

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
    pub host_workspace_path: PathBuf,
    pub additional_mounts: Vec<Mount>,
    pub network: Option<String>,
}

impl ContainerConfig {
    pub fn from_config(
        config_toml: Option<&config::Container>,
        local_workspace_path: &std::path::Path,
    ) -> Result<ContainerConfig> {
        // TODO this wrong is we're running in the container
        let host_workspace_path = config_toml
            .and_then(|c| c.workspace_mount_path.as_deref())
            .unwrap_or(local_workspace_path)
            .to_path_buf();

        let additional_mounts_toml = config_toml
            .and_then(|c| c.additional_mounts.as_deref())
            .unwrap_or_default();

        let mut mounts = Vec::new();
        for mount_toml in additional_mounts_toml {
            mounts.push(Mount {
                typ: Some(MountTypeEnum::BIND),
                source: Some(Self::mount_path(&mount_toml.source, &host_workspace_path)),
                target: Some(Self::mount_path(&mount_toml.target, &host_workspace_path)),
                bind_options: Some(MountBindOptions {
                    create_mountpoint: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }

        Ok(Self {
            host_workspace_path,
            additional_mounts: mounts,
            network: config_toml.and_then(|c| c.network.clone()),
        })
    }

    fn mount_path(path: &Path, host_workspace_path: &Path) -> String {
        // TODO consider variable substituion
        let resolved_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            host_workspace_path.join(path)
        };

        resolved_path.to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::models::MountTypeEnum;
    use std::path::Path;

    fn container_cfg(
        workspace_mount_path: Option<&str>,
        mounts: Option<Vec<config::Mount>>,
    ) -> config::Container {
        config::Container {
            workspace_mount_path: workspace_mount_path.map(PathBuf::from),
            network: None,
            additional_mounts: mounts,
        }
    }

    #[test]
    fn test_relative_source_joined_with_workspace_mount_path() {
        let cfg = container_cfg(
            Some("/host/workspace"),
            Some(vec![config::Mount {
                source: PathBuf::from("../.aws/config"),
                target: PathBuf::from("/home/k/.aws/config"),
            }]),
        );
        let result = ContainerConfig::from_config(Some(&cfg), Path::new("/local/ws")).unwrap();

        assert_eq!(result.additional_mounts.len(), 1);
        assert_eq!(
            result.additional_mounts[0].source.as_deref(),
            Some("/host/workspace/../.aws/config")
        );
        assert_eq!(
            result.additional_mounts[0].target.as_deref(),
            Some("/home/k/.aws/config")
        );
    }

    #[test]
    fn test_host_path_falls_back_to_local_workspace_path() {
        let cfg = container_cfg(
            None,
            Some(vec![config::Mount {
                source: PathBuf::from("../.aws/config"),
                target: PathBuf::from("/home/k/.aws/config"),
            }]),
        );
        let result = ContainerConfig::from_config(Some(&cfg), Path::new("/local/ws")).unwrap();

        assert_eq!(
            result.additional_mounts[0].source.as_deref(),
            Some("/local/ws/../.aws/config")
        );
    }

    #[test]
    fn test_mount_with_absolute_path_passes_through_unchanged() {
        let cfg = container_cfg(
            None,
            Some(vec![config::Mount {
                source: PathBuf::from("/absolute/source"),
                target: PathBuf::from("/absolute/target"),
            }]),
        );
        let result = ContainerConfig::from_config(Some(&cfg), Path::new("/local/ws")).unwrap();

        assert_eq!(
            result.additional_mounts[0].source.as_deref(),
            Some("/absolute/source")
        );
        assert_eq!(
            result.additional_mounts[0].target.as_deref(),
            Some("/absolute/target")
        );
    }

    #[test]
    fn test_no_additional_mounts_produces_empty_vec() {
        let cfg = container_cfg(None, None);
        let result = ContainerConfig::from_config(Some(&cfg), Path::new("/local/ws")).unwrap();
        assert!(result.additional_mounts.is_empty());
    }

    #[test]
    fn test_no_config_produces_empty_mounts() {
        let result = ContainerConfig::from_config(None, Path::new("/local/ws")).unwrap();
        assert!(result.additional_mounts.is_empty());
    }

    #[test]
    fn test_mounts_are_bind_type_with_no_create_mountpoint() {
        let cfg = container_cfg(
            None,
            Some(vec![config::Mount {
                source: PathBuf::from("/src"),
                target: PathBuf::from("/tgt"),
            }]),
        );
        let result = ContainerConfig::from_config(Some(&cfg), Path::new("/ws")).unwrap();

        let mount = &result.additional_mounts[0];
        assert_eq!(mount.typ, Some(MountTypeEnum::BIND));
        assert_eq!(
            mount
                .bind_options
                .as_ref()
                .and_then(|o| o.create_mountpoint),
            Some(false)
        );
    }
}
