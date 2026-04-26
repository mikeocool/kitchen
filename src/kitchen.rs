use std::path::PathBuf;

pub struct Kitchen {
    pub workspace_path: PathBuf,
    pub name: String,
}

impl Kitchen {
    pub fn container_name(&self) -> String {
        format!("{}-kitchen", self.name)
    }
}
