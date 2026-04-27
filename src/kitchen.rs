use std::path::PathBuf;

use crate::config::KitchenToml;

pub struct Kitchen {
    pub workspace_path: PathBuf,
    pub name: String,
    pub config: Option<KitchenToml>,
}

impl Kitchen {
    pub fn container_name(&self) -> String {
        format!("{}-kitchen", self.name)
    }

    pub fn container_workspace_path(&self) -> String {
        format!("/workspaces/{}", self.name)
    }

    pub fn workspace_host_path(&self) -> String {
        self.config
            .as_ref()
            .and_then(|c| c.container.as_ref())
            .and_then(|c| c.workspace_mount_path.as_deref())
            .unwrap_or_else(|| self.workspace_path.to_str().unwrap_or_default())
            .to_string()
    }

    pub fn kitchen_workspace_env(&self) -> String {
        format!("KITCHEN_WORKSPACE={}", self.container_workspace_path())
    }
}
