# Make Extensions Dynamic & Toggleable

## Context

Today's extensions (dotfiles, tailscale, pitchfork) are wired in by hand: `extensions::onstart()` and `extensions::poststart()` (`src/extensions/mod.rs:7-22`) call each module's hook explicitly, and `image::build_context_tar()` (`src/image.rs:72`) hard-codes `tailscale::image_context()`. New hooks (install, future ones) require editing every dispatcher. Extension configs are bolted onto `KitchenConfig` directly (e.g. `dotfiles_repo`, `dotfiles_install_cmd` on `kitchen.rs:41-42`), which won't scale as more extensions are added.

We want:

1. `KitchenConfig` carries a `Vec<Box<dyn Extension>>` of active extensions.
2. Each hook point iterates the vec; extensions only do work for hooks they care about.
3. Each extension owns its own config, parsed from its own TOML section.
4. Extensions are default-on; toggleable off via TOML.
5. Easy to add new extensions and new hook points without touching dispatchers.

## Decisions

- **Discovery:** default-on registry. Every extension in `REGISTRY` runs unless listed in top-level `disabled_extensions`.
- **TOML namespace:** flat — `[dotfiles]`, not `[extensions.dotfiles]`.
- **Ordering:** `REGISTRY` order. Code-defined, deterministic, doesn't depend on user TOML key order. Tailscale before pitchfork is enforced by registry order.
- **Async strategy:** `async-trait` crate. One-line dependency, ergonomic; the alternative is hand-rolled `Pin<Box<dyn Future>>` returns, which buys nothing here.
- **Failure mode:** stop the chain on first error (matches current behavior).

## Design

### The `Extension` trait

Every extension is a struct that owns its parsed config. The trait provides default no-op implementations of every hook, so each extension only writes the hooks it cares about. Adding a new hook = one trait method with a default + one dispatcher loop, no edits to existing extensions.

```rust
// src/extensions/mod.rs
use async_trait::async_trait;
use std::collections::HashSet;

use crate::config::KitchenToml;
use crate::image::ContextFile;
use crate::kitchen::KitchenConfig;

pub mod dotfiles;
pub mod pitchfork;
pub mod tailscale;

#[async_trait]
pub trait Extension: Send + Sync {
    fn name(&self) -> &'static str;

    fn image_context(&self, _k: &KitchenConfig) -> Vec<ContextFile> { vec![] }
    fn install(&self, _k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
    async fn onstart(&self, _k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
    async fn poststart(&self, _k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
    // future: pre_build, container_mounts, container_env, ondown, ...
}

type Builder = fn(&toml::Value) -> Result<Box<dyn Extension>, Box<dyn std::error::Error>>;

// Order here = execution order.
const REGISTRY: &[(&str, Builder)] = &[
    ("dotfiles",  |v| Ok(Box::new(dotfiles::Dotfiles::from_toml(v)?))),
    ("tailscale", |v| Ok(Box::new(tailscale::Tailscale::from_toml(v)?))),
    ("pitchfork", |v| Ok(Box::new(pitchfork::Pitchfork::from_toml(v)?))),
];

pub fn build(
    toml: Option<&KitchenToml>,
) -> Result<Vec<Box<dyn Extension>>, Box<dyn std::error::Error>> {
    let disabled: HashSet<&str> = toml
        .and_then(|t| t.disabled_extensions.as_deref())
        .unwrap_or(&[])
        .iter()
        .map(String::as_str)
        .collect();

    let known: HashSet<&str> = REGISTRY.iter().map(|(n, _)| *n).collect();
    let empty_table = toml::Table::new();
    let configs = toml.map(|t| &t.extension_configs).unwrap_or(&empty_table);

    // Typo protection: anything in extension_configs that isn't a registered extension.
    for key in configs.keys() {
        if !known.contains(key.as_str()) {
            return Err(format!("unknown extension config section: [{key}]").into());
        }
    }

    let empty_value = toml::Value::Table(toml::Table::new());
    let mut out = Vec::with_capacity(REGISTRY.len());
    for (name, builder) in REGISTRY {
        if disabled.contains(name) { continue; }
        let cfg = configs.get(*name).unwrap_or(&empty_value);
        out.push(builder(cfg)?);
    }
    Ok(out)
}

pub async fn onstart(k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running kitchen onstart hooks...");
    for ext in &k.extensions { ext.onstart(k).await?; }
    Ok(())
}

pub async fn poststart(k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running kitchen poststart hooks...");
    for ext in &k.extensions { ext.poststart(k).await?; }
    Ok(())
}
```

### `src/config.rs` — flat sections via `#[serde(flatten)]`

`#[serde(flatten)]` parses the typed fields first (`name`, `container`, `features`, `disabled_extensions`); whatever's left lands in `extension_configs` keyed by section name. So `[dotfiles]` and `[tailscale]` end up in `extension_configs`.

```rust
#[derive(Deserialize, Debug)]
pub struct KitchenToml {
    pub name: Option<String>,
    pub container: Option<Container>,
    pub features: Option<Features>,
    pub disabled_extensions: Option<Vec<String>>,

    #[serde(flatten)]
    pub extension_configs: toml::Table,
}
```

The legacy top-level `dotfiles_repo` / `dotfiles_install_cmd` fields get removed.

### `src/kitchen.rs` — `KitchenConfig::from_workspace`

```rust
pub struct KitchenConfig {
    pub name: String,
    pub local_workspace_path: PathBuf,
    pub container_workspace_path: String,
    pub container: ContainerConfig,
    pub extensions: Vec<Box<dyn Extension>>,
}

impl KitchenConfig {
    pub fn from_workspace(
        workspace: &Option<PathBuf>,
    ) -> Result<KitchenConfig, Box<dyn std::error::Error>> {
        let local_workspace_path = match workspace {
            Some(ws) => std::fs::canonicalize(ws).unwrap_or_else(|_| ws.clone()),
            None => std::env::current_dir()?,
        };

        let config_toml = config::load(&local_workspace_path)?;
        let config_toml = config_toml.as_ref();

        let workspace_dir_name = local_workspace_path
            .file_name()
            .ok_or("workspace path has no final component")?
            .to_string_lossy()
            .into_owned();

        let name = config_toml
            .and_then(|c| c.name.clone())
            .unwrap_or(workspace_dir_name);

        let container_workspace_path = format!("/workspaces/{}", name);

        let container = ContainerConfig::from_config(
            config_toml.and_then(|c| c.container.as_ref()),
            local_workspace_path.as_path(),
        );

        let extensions = extensions::build(config_toml)?;

        Ok(KitchenConfig {
            name,
            local_workspace_path,
            container_workspace_path,
            container,
            extensions,
        })
    }
}
```

The `dotfiles_repo` / `dotfiles_install_cmd` fields come off `KitchenConfig` — that data now lives on the `Dotfiles` extension.

### Per-extension shape — `src/extensions/dotfiles.rs`

```rust
use async_trait::async_trait;
use serde::Deserialize;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::extensions::Extension;
use crate::kitchen::KitchenConfig;

const SCRIPT: &str = include_str!("../../resources/provision/dotfiles.sh");

pub struct Dotfiles {
    pub repo: Option<String>,
    pub install_cmd: Option<String>,
}

#[derive(Deserialize, Default)]
struct Toml {
    repo: Option<String>,
    install_cmd: Option<String>,
}

impl Dotfiles {
    pub fn from_toml(v: &toml::Value) -> Result<Self, Box<dyn std::error::Error>> {
        let cfg: Toml = v.clone().try_into().unwrap_or_default();
        Ok(Self { repo: cfg.repo, install_cmd: cfg.install_cmd })
    }
}

#[async_trait]
impl Extension for Dotfiles {
    fn name(&self) -> &'static str { "dotfiles" }

    async fn onstart(&self, _k: &KitchenConfig) -> Result<(), Box<dyn std::error::Error>> {
        let Some(repo) = &self.repo else { return Ok(()); };
        let install_cmd = self.install_cmd.as_deref().unwrap_or("");

        let mut child = Command::new("sh")
            .args(["-s", "--", repo, install_cmd])
            .stdin(Stdio::piped())
            .spawn()?;
        child.stdin.as_mut().unwrap().write_all(SCRIPT.as_bytes())?;
        if !child.wait()?.success() {
            return Err("dotfiles provisioning failed".into());
        }
        Ok(())
    }
}
```

`Tailscale` and `Pitchfork` follow the same pattern: a struct (possibly unit), a `from_toml` factory, and `impl Extension` overriding only the hooks they need (e.g. `Tailscale` overrides `image_context` + `poststart`; `Pitchfork` overrides `onstart`).

### `src/image.rs:72` — image_context dispatch

```rust
for ext in &kitchen.extensions {
    files.extend(ext.image_context(kitchen));
}
```

Replaces the hard-coded `files.extend(tailscale::image_context(kitchen));`.

## Resulting TOML

Minimal — all three extensions enabled, dotfiles is a no-op (no repo configured):
```toml
name = "kitchen"
```

Configured:
```toml
name = "kitchen"

[dotfiles]
repo = "https://github.com/me/dotfiles"
install_cmd = "./install.sh"
```

Disabling:
```toml
name = "kitchen"
disabled_extensions = ["tailscale", "pitchfork"]

[dotfiles]
repo = "..."
```

Typo gets caught:
```toml
[dotfile]   # error: unknown extension config section: [dotfile]
repo = "..."
```

## Files to change

- `Cargo.toml` — add `async-trait`.
- `src/extensions/mod.rs` — define `Extension` trait, `REGISTRY`, `build()`, generic dispatchers.
- `src/extensions/dotfiles.rs` — convert free fn to `Dotfiles` struct + `impl Extension` + `from_toml`.
- `src/extensions/tailscale.rs` — same; struct holds no config, overrides `image_context` and `poststart`.
- `src/extensions/pitchfork.rs` — same; struct holds no config, overrides `onstart`.
- `src/config.rs` — drop `dotfiles_repo` / `dotfiles_install_cmd`; add `disabled_extensions`; add flattened `extension_configs: toml::Table`.
- `src/kitchen.rs` — drop dotfiles fields from `KitchenConfig`; add `extensions: Vec<Box<dyn Extension>>`; call `extensions::build` from `from_workspace`.
- `src/image.rs:72` — replace hard-coded `tailscale::image_context` with iteration over `kitchen.extensions`.
- Migrate fixtures: `.kitchen.toml`, `fixtures/testws/.kitchen.toml`, `fixtures/compose-test/.kitchen.toml` — move flat dotfiles fields into `[dotfiles]` section.
- **First step of execution:** copy this plan to `plans/20260429-dynamic-extensions.md` so it lives in the repo.

## Verification

- `cargo check && cargo build`.
- `kitchen up` in `/workspaces/kitchen` — dotfiles provisions, tailscale + pitchfork run as today.
- Add `disabled_extensions = ["tailscale"]` to a fixture; confirm `tailscale up` does not run in poststart.
- Set up a `.kitchen.toml` with no `[dotfiles]` section; confirm dotfiles hook is a no-op (no repo configured) and tailscale/pitchfork still run.
- Add a `[dotfile]` typo section; confirm `kitchen up` fails with "unknown extension config section: [dotfile]".
