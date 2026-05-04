use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use eyre::{Result, eyre};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

pub enum ScriptInput {
    Script(String),
    Command(String, Vec<String>),
}

pub struct ScriptRunner {
    input: ScriptInput,
    sudo: bool,
    shell: String,
    working_dir: Option<PathBuf>,
    env: HashMap<String, String>,
    timeout: Option<Duration>,
    label: Option<String>,
}

impl ScriptRunner {
    pub fn script(script: impl Into<String>) -> Self {
        Self::new(ScriptInput::Script(script.into()))
    }

    pub fn command(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::new(ScriptInput::Command(
            program.into(),
            args.into_iter().map(Into::into).collect(),
        ))
    }

    pub fn new(input: ScriptInput) -> Self {
        Self {
            input,
            sudo: false,
            shell: "sh".into(),
            working_dir: None,
            env: HashMap::new(),
            timeout: None,
            label: None,
        }
    }

    pub fn sudo(mut self) -> Self {
        self.sudo = true;
        self
    }

    pub fn shell(mut self, shell: impl Into<String>) -> Self {
        self.shell = shell.into();
        self
    }

    pub fn working_dir(mut self, working_dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.env.insert(key.into(), val.into());
        self
    }

    pub fn timeout(mut self, dur: Duration) -> Self {
        self.timeout = Some(dur);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub async fn run(&self) -> Result<()> {
        if let Some(dur) = self.timeout {
            timeout(dur, self.execute()).await.map_err(|_| {
                eyre!(
                    "{} timed out after {:.0?}",
                    self.label.as_deref().unwrap_or("script"),
                    dur
                )
            })?
        } else {
            self.execute().await
        }
    }

    async fn execute(&self) -> Result<()> {
        let label = self.label.as_deref().unwrap_or("script");
        (match &self.input {
            ScriptInput::Script(_) => "script",
            ScriptInput::Command(prog, _) => prog.as_str(),
        });

        if let Some(l) = &self.label {
            println!("==> {l}");
        }

        let mut cmd = match &self.input {
            ScriptInput::Script(_) => {
                // Pipe the script to [sudo] sh -s via stdin — no quoting needed.
                let (program, args) = if self.sudo {
                    ("sudo", vec![self.shell.as_str(), "-s"])
                } else {
                    (self.shell.as_str(), vec!["-s"])
                };
                let mut cmd = Command::new(program);
                cmd.args(&args).stdin(Stdio::piped());
                cmd
            }
            ScriptInput::Command(program, args) => {
                // Run the binary directly — no shell, no stdin.
                let mut cmd = if self.sudo {
                    let mut c = Command::new("sudo");
                    c.arg(program);
                    c
                } else {
                    Command::new(program)
                };
                cmd.args(args).stdin(Stdio::null());
                cmd
            }
        };

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&self.env);

        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn()?;

        // For script input: write to stdin then close it so the shell sees EOF.
        if let ScriptInput::Script(script) = &self.input {
            let mut stdin = child.stdin.take().expect("stdin is piped");
            let script = script.clone();
            tokio::spawn(async move {
                let _ = stdin.write_all(script.as_bytes()).await;
                // stdin drops here, sending EOF
            });
        }

        // Stream stdout and stderr concurrently — neither blocks the other.
        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr = child.stderr.take().expect("stderr is piped");

        let stdout_task = async {
            let mut lines = BufReader::new(stdout).lines();
            while let Some(line) = lines.next_line().await? {
                println!("{line}");
            }
            Ok::<_, eyre::Report>(())
        };

        let stderr_task = async {
            let mut lines = BufReader::new(stderr).lines();
            while let Some(line) = lines.next_line().await? {
                eprintln!("{line}");
            }
            Ok::<_, eyre::Report>(())
        };

        tokio::try_join!(stdout_task, stderr_task)?;

        let status = child.wait().await?;
        if !status.success() {
            let code = status.code().unwrap_or(-1);
            return Err(eyre!("{label} exited with status {code}"));
        }

        Ok(())
    }
}
