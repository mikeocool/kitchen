use async_trait::async_trait;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

use eyre::{Result, eyre};

use crate::extensions::Extension;
use crate::image::ContextFile;
use crate::kitchen::KitchenConfig;

const PITCHFORK_TOML: &[u8] = include_bytes!("../../resources/tailscale/pitchfork.toml");

pub struct Tailscale {}

impl Tailscale {
    pub fn from_toml(_v: &toml::Value) -> Result<Self> {
        Ok(Self {})
    }
}

#[async_trait]
impl Extension for Tailscale {
    fn name(&self) -> &'static str {
        "tailscale"
    }

    fn image_context(&self, _k: &KitchenConfig) -> Result<Vec<ContextFile>> {
        Ok(vec![ContextFile::new(
            "tailscale/pitchfork.toml",
            PITCHFORK_TOML,
        )])
    }

    async fn install(&self, _k: &KitchenConfig) -> Result<()> {
        // get and run tailscale install script, if it's not already installed
        // TODO put daemon in place
        Ok(())
    }

    async fn poststart(&self, _k: &KitchenConfig) -> Result<()> {
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
            return Err(eyre!("tailscale up --ssh failed"));
        }

        Ok(())
    }
}
