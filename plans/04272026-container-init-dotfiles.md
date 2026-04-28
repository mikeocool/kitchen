# Plan: `container-provision` command — dotfiles

**Goal**: Add a `container-provision` subcommand that runs inside the container at startup.
Initially it handles dotfiles (clone/pull/install). The module structure is designed to
accommodate future provisioning steps (ssh keys, tool installs, secrets, etc.) without
restructuring.

---

## File layout

```
resources/
  dotfiles.sh             — shell script: git check, clone vs. pull, run install if changed
src/
  main.rs                 — add ContainerProvision variant + dispatch function
  provision/
    mod.rs                — pub async fn run() — orchestrates steps in order
    dotfiles.rs           — embeds the script, passes config values, invokes sh
```

Git logic lives entirely in `dotfiles.sh`. `dotfiles.rs` is a thin wrapper that reads config
and invokes the script — no git-specific Rust code. The script is embedded in the binary at
compile time with `include_str!` and piped to `sh -s`, so it doesn't need to be a separate file
inside the container image.

---

## Command wiring (`src/main.rs`)

`ContainerProvision` takes no workspace argument — it runs inside the container and resolves the
workspace from `KITCHEN_WORKSPACE` (set by container.rs at `run` time).

```rust
mod provision;  // add alongside mod container;

#[derive(Subcommand)]
enum Commands {
    Up { workspace: Option<PathBuf> },
    Down { workspace: Option<PathBuf> },
    Build { workspace: Option<PathBuf> },
    ContainerProvision,
}

// Dispatch in main():
match &cli.command {
    Some(Commands::Up { workspace })       => up(workspace).await,
    Some(Commands::Down { workspace })     => down(workspace).await,
    Some(Commands::Build { workspace })    => build(workspace).await,
    Some(Commands::ContainerProvision)     => container_provision().await,
    None => {}
}

async fn container_provision() {
    let workspace_path = std::env::var("KITCHEN_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("no working directory"));

    let config = config::load(&workspace_path).unwrap_or_else(|e| {
        eprintln!("Error loading config: {e}");
        std::process::exit(1);
    });

    let kitchen = Kitchen {
        name: workspace_path
            .file_name()
            .expect("workspace path has no final component")
            .to_string_lossy()
            .into_owned(),
        workspace_path,
        config,
    };

    if let Err(e) = provision::run(&kitchen).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
```

Also add `mod provision;` alongside the existing `mod container;`.

---

## `src/provision/mod.rs`

Orchestrates steps. Each future capability is a module call added here.

```rust
use crate::kitchen::Kitchen;

mod dotfiles;

pub async fn run(kitchen: &Kitchen) -> Result<(), Box<dyn std::error::Error>> {
    dotfiles::provision(kitchen)?;
    // future: ssh_keys::provision(kitchen)?;
    // future: tools::provision(kitchen)?;
    Ok(())
}
```

`run` is `async` for consistency with the rest of the codebase and to allow future steps that
need async (downloading tool binaries, hitting APIs, etc.).

---

## `resources/dotfiles.sh`

All git logic lives here. Invoked as `sh -s -- <repo> <install_cmd>`.

`$1` — git repo URL (always provided when script is called)
`$2` — install command (may be empty string; script skips install if so)

```sh
#!/usr/bin/env sh
set -e

REPO="$1"
INSTALL_CMD="$2"
DOTFILES_DIR="${HOME}/dotfiles"

if ! command -v git > /dev/null 2>&1; then
    echo "Error: git is not installed but dotfiles_repo is configured" >&2
    exit 1
fi

if [ -d "$DOTFILES_DIR" ]; then
    cd "$DOTFILES_DIR"
    BEFORE=$(git rev-parse HEAD)
    git pull
    AFTER=$(git rev-parse HEAD)
    if [ "$BEFORE" = "$AFTER" ]; then
        exit 0
    fi
else
    git clone "$REPO" "$DOTFILES_DIR"
    cd "$DOTFILES_DIR"
fi

if [ -n "$INSTALL_CMD" ]; then
    sh -c "$INSTALL_CMD"
fi
```

`set -e` means any git failure (no network, auth error, bad repo URL) exits non-zero and
surfaces as an error in the Rust caller — no explicit error handling needed in the script itself.

---

## `src/provision/dotfiles.rs`

Embeds the script, reads config, and invokes `sh -s`. No git-specific Rust.

```rust
use std::io::Write;
use std::process::{Command, Stdio};

use crate::kitchen::Kitchen;

const SCRIPT: &str = include_str!("../../resources/dotfiles.sh");

pub fn provision(kitchen: &Kitchen) -> Result<(), Box<dyn std::error::Error>> {
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

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(SCRIPT.as_bytes())?;

    let status = child.wait()?;
    if !status.success() {
        return Err("dotfiles provisioning failed".into());
    }

    Ok(())
}
```

---

## Design notes

- **`rev-parse HEAD` before/after** is the reliable way to detect whether `git pull` brought in
  commits — no stdout string parsing, works regardless of git's output locale or version.
- **`sh -s -- args`** pipes the script to a new shell via stdin and passes arguments normally.
  The script is compiled into the binary via `include_str!`, so no extra file needs to be present
  inside the container image.
- **`set -e`** in the script means git failures propagate as non-zero exit without any explicit
  error handling — `child.wait()` sees the non-zero exit and the Rust caller returns an error.
- **`sh -c "$INSTALL_CMD"`** for the install step lets users write compound commands
  (`make install && source ~/.profile`) without kitchen needing to parse shell syntax.
- `provision::run` owns step ordering. Steps are not parallelised — dotfiles must finish before
  tool installs that depend on them.

---

## Steps in order

1. Create `resources/dotfiles.sh`.
2. Add `mod provision;` to `main.rs`.
3. Add `ContainerProvision` variant to `Commands` enum in `main.rs`.
4. Add `container_provision()` function and dispatch arm in `main()`.
5. Create `src/provision/mod.rs`.
6. Create `src/provision/dotfiles.rs`.
7. Smoke-test: run `kitchen container-provision` inside a container with `dotfiles_repo` set in
   `.kitchen.toml` — verify clone on first run, no install on second run (no new commits), install
   triggered when repo has new commits.
