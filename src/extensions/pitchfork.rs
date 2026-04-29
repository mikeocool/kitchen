use crate::kitchen::KitchenConfig;
use std::io::Write;
use std::process::{Command, Stdio};

const SCRIPT: &str = include_str!("../../resources/pitchfork/onstart.sh");

pub fn onstart(_kitchen: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("sudo")
        .args(["sh", "-s"])
        .stdin(Stdio::piped())
        .spawn()?;

    child.stdin.as_mut().unwrap().write_all(SCRIPT.as_bytes())?;

    let status = child.wait()?;
    if !status.success() {
        return Err("pitchfork onstart failed".into());
    }

    Ok(())
}
