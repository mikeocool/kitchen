use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

use crate::image::ContextFile;
use crate::kitchen::KitchenConfig;

const PITCHFORK_TOML: &[u8] = include_bytes!("../../resources/tailscale/pitchfork.toml");

pub fn image_context(_kitchen: &KitchenConfig) -> Vec<ContextFile> {
    vec![ContextFile::new("tailscale/pitchfork.toml", PITCHFORK_TOML)]
}

pub fn install(_kitchen: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
    // get and run tailscale install script, if it's not already installed
    // TODO put daemon in place
    Ok(())
}

pub fn poststart(_kitchen: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("sudo")
        .args(["tailscale", "up", "--ssh"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_thread = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if let Ok(line) = line {
                println!("{}", line);
            }
        }
    });

    let stderr_thread = thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            if let Ok(line) = line {
                eprintln!("{}", line);
            }
        }
    });

    stdout_thread.join().ok();
    stderr_thread.join().ok();

    let status = child.wait()?;
    if !status.success() {
        return Err("tailscale up --ssh failed".into());
    }

    Ok(())
}
