use std::io::Write;
use std::process::{Command, Stdio};

use crate::kitchen::Kitchen;

const SCRIPT: &str = include_str!("../../resources/provision/dotfiles.sh");

pub fn onstart(kitchen: &Kitchen) -> Result<(), Box<dyn std::error::Error>> {
    let config = match &kitchen.config {
        Some(c) => c,
        None => return Ok(()),
    };

    let repo = match &config.dotfiles_repo {
        Some(r) => r.as_str(),
        None => return Ok(()),
    };

    let install_cmd = config.dotfiles_install_cmd.as_deref().unwrap_or("");

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
