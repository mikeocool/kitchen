use std::io::Write;
use std::process::{Command, Stdio};

use crate::kitchen::KitchenConfig;

const SCRIPT: &str = include_str!("../../resources/provision/dotfiles.sh");

pub fn onstart(kitchen: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
    let repo = match &kitchen.dotfiles_repo {
        Some(r) => r.as_str(),
        None => return Ok(()),
    };

    let install_cmd = kitchen.dotfiles_install_cmd.as_deref().unwrap_or("");

    let mut child = Command::new("sh")
        .args(["-s", "--", repo, install_cmd])
        .stdin(Stdio::piped())
        .spawn()?;

    child.stdin.as_mut().unwrap().write_all(SCRIPT.as_bytes())?;

    let status = child.wait()?;
    if !status.success() {
        return Err("dotfiles provisioning failed".into());
    }

    Ok(())
}
