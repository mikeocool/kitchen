# Plan: `cmd` module — `run_script`

## Goal

A standalone `cmd` module (not an Extension) providing a reusable, ergonomic API for running shell scripts on the host, streaming output to the user in real time, and surfacing non-zero exit codes as eyre errors.

---

## Module location

```
src/cmd/mod.rs
```

Declared in `src/main.rs` as `mod cmd;`.

No new Cargo dependencies — `tokio` with full features is already present.

---

## Public API

A builder struct for full control, plus a convenience free function for the common case:

```rust
// src/cmd/mod.rs

pub async fn run_script(script: &str) -> Result<()> {
    ScriptRunner::new(script).run().await
}

// Full control
ScriptRunner::new(script)
    .sudo()
    .shell("bash")
    .working_dir("/some/path")
    .env("MY_VAR", "value")
    .label("Installing mise")
    .timeout(Duration::from_secs(120))
    .run()
    .await?;
```

---

## `ScriptRunner` struct + builder

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use eyre::{eyre, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;
use std::process::Stdio;

pub struct ScriptRunner {
    script: String,
    sudo: bool,
    shell: String,
    working_dir: Option<PathBuf>,
    env: HashMap<String, String>,
    timeout: Option<Duration>,
    label: Option<String>,
}

impl ScriptRunner {
    pub fn new(script: impl Into<String>) -> Self {
        Self {
            script: script.into(),
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

    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
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
            timeout(dur, self.execute())
                .await
                .map_err(|_| eyre!(
                    "{} timed out after {:.0?}",
                    self.label.as_deref().unwrap_or("script"),
                    dur
                ))?
        } else {
            self.execute().await
        }
    }
}
```

---

## `execute()` — streaming implementation

The script is piped to the shell's stdin (mirrors the existing `dotfiles.rs` / `pitchfork.rs` pattern). This avoids temp files and shell-escaping the script content entirely.

```rust
impl ScriptRunner {
    async fn execute(&self) -> Result<()> {
        let label = self.label.as_deref().unwrap_or("script");

        if let Some(l) = &self.label {
            println!("==> {l}");
        }

        // Build: [sudo] sh -s  (or bash -s, etc.)
        let (program, mut args) = if self.sudo {
            ("sudo", vec![self.shell.as_str(), "-s"])
        } else {
            (self.shell.as_str(), vec!["-s"])
        };

        let mut cmd = Command::new(program);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&self.env);

        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn()?;

        // Write the script to stdin, then close it so the shell sees EOF.
        let mut stdin = child.stdin.take().expect("stdin is piped");
        let script = self.script.clone();
        tokio::spawn(async move {
            let _ = stdin.write_all(script.as_bytes()).await;
            // stdin drops here, sending EOF
        });

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
```

---

## Options — rationale and design

### `sudo` (bool, default `false`)

Prepends `sudo` to the shell invocation. The script reaches stdin the same way — no change to the script itself required.

```rust
// Without sudo:  sh -s
// With sudo:    sudo sh -s
let program = if self.sudo { "sudo" } else { &self.shell };
```

**Considered:** a `sudo -n` dry-run first to fail fast if the user has no sudo access. Decided against it — `sudo` itself will print a clear error and we'd get a non-zero exit either way. Keep it simple.

### `shell` (String, default `"sh"`)

Lets callers use bash-specific features (`set -euo pipefail`, arrays, process substitution). Because the script is fed via stdin rather than `-c "..."`, no quoting is needed regardless of shell.

```rust
// Caller wants bash strict mode at the top of their script:
ScriptRunner::new("set -euo pipefail\n...\n")
    .shell("bash")
    .run()
    .await?;
```

### `working_dir` (Option<PathBuf>)

Passed straight to `Command::current_dir()`. Inherits the process CWD if unset.

```rust
ScriptRunner::new(script)
    .working_dir(&kitchen.local_workspace_path)
    .run()
    .await?;
```

### `env` (HashMap<String, String>)

Extends (does not replace) the inherited environment. Useful for passing credentials or flags without embedding them in the script string.

```rust
ScriptRunner::new(script)
    .env("DOTFILES_REPO", repo)
    .env("INSTALL_CMD", install_cmd)
    .run()
    .await?;
```

This is an alternative to the current `dotfiles.sh` approach of passing repo/install_cmd as `$1`/`$2` positional args — either style works.

### `label` (Option<String>)

Printed as `==> <label>` before the first output line. Included in error messages so call sites don't need to `.wrap_err()`, though they may still do so for additional context.

```rust
// Output:
// ==> Installing mise
// <stdout from script>
// error: Installing mise exited with status 1
```

### `timeout` (Option<Duration>)

Wraps `execute()` in `tokio::time::timeout`. On expiry the future is dropped (which closes the child's I/O handles), then the error is returned. For hard process termination, add an explicit `child.kill().await` before returning — defer to v2 if needed.

```rust
ScriptRunner::new(slow_script)
    .timeout(Duration::from_secs(300))
    .label("long provisioner")
    .run()
    .await?;
// On timeout: error: long provisioner timed out after 300s
```

---

## Options deferred to v2

| Option                             | Why defer                                                                                                                      |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `capture_output -> Result<String>` | Different return type — separate function, not a flag on this builder                                                          |
| `stdin_passthrough`                | Needed for interactive scripts; conflicts with the piped-stdin approach and needs raw-mode handling (see `container/shell.rs`) |
| `run_as(user)`                     | `sudo -u <user>` is niche; add when there is a concrete caller                                                                 |
| `env_clear`                        | Security hardening; defer until there is a concrete reason                                                                     |
| Hard kill on timeout               | `child.kill().await` before returning; defer unless a script is known to ignore SIGTERM                                        |

---

## Using it from an extension

Extensions that currently spawn child processes directly (`dotfiles.rs`, `pitchfork.rs`) can migrate to `ScriptRunner`:

```rust
// Before (dotfiles.rs onstart):
let mut child = Command::new("sh")
    .args(["-s", "--", repo, install_cmd])
    .stdin(Stdio::piped())
    .spawn()?;
child.stdin.as_mut().unwrap().write_all(SCRIPT.as_bytes())?;
if !child.wait()?.success() {
    return Err(eyre!("dotfiles provisioning failed"));
}

// After:
cmd::ScriptRunner::new(SCRIPT)
    .env("DOTFILES_REPO", repo)
    .env("INSTALL_CMD", install_cmd)
    .label("dotfiles")
    .run()
    .await?;
```

---

## Files to create / modify

| File             | Action                                                                |
| ---------------- | --------------------------------------------------------------------- |
| `src/cmd/mod.rs` | Create — `ScriptRunner`, builder methods, `execute()`, `run_script()` |
| `src/main.rs`    | Add `mod cmd;`                                                        |
| `Cargo.toml`     | No changes needed                                                     |
