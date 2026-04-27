use std::path::PathBuf;

pub struct Kitchen {
    pub workspace_path: PathBuf,
    pub name: String,
}

impl Kitchen {
    pub fn container_name(&self) -> String {
        format!("{}-kitchen", self.name)
    }

    pub fn workspace_mount(&self) -> String {
        format!("{}:/workspace/{}", self.workspace_path.display(), self.name)
    }

    pub fn kitchen_workspace_env(&self) -> String {
        format!("KITCHEN_WORKSPACE=/workspace/{}", self.name)
    }
}
