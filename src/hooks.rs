use eyre::Result;

use crate::KitchenConfig;
use crate::cmd::ScriptRunner;

pub async fn poststart(kitchen: &KitchenConfig) -> Result<()> {
    let hook_path = kitchen
        .container_workspace_path()
        .join(".kitchen")
        .join("hooks")
        .join("poststart");

    if hook_path.exists() {
        ScriptRunner::command(hook_path, vec![] as Vec<&str>)
            .label("Running poststart hook")
            .run()
            .await?;
    }

    Ok(())
}
