# Plan: Implement `docker run` in the `up` command

**Goal**: After building the image, create and start a detached container equivalent to:

```
docker run -d \
  --name <container_name> \
  --hostname <container_name> \
  -e "KITCHEN_WORKSPACE=/workspace/<kitchen.name>" \
  -v "<kitchen.workspace_path>:/workspace/<kitchen.name>" \
  <container_name>
```

---

## File organisation

```
src/
  main.rs        — CLI wiring, calls container::run after image::build
  kitchen.rs     — add workspace_mount() and kitchen_workspace_env() methods
  image.rs       — unchanged
  container.rs   — new: create_and_start function (mirrors image.rs pattern)
```

No new directories are needed. `container.rs` keeps container lifecycle separate from image
building, mirroring the existing `image.rs` split.

---

## `src/kitchen.rs` changes

Add two helper methods so callers never construct paths or env strings by hand:

```rust
pub fn workspace_mount(&self) -> String {
    // "host_path:/workspace/name"  — the -v bind-mount string
    format!("{}:/workspace/{}", self.workspace_path.display(), self.name)
}

pub fn kitchen_workspace_env(&self) -> String {
    format!("KITCHEN_WORKSPACE=/workspace/{}", self.name)
}
```

---

## `src/image.rs` changes

Update the call site in `build` to accept and use `container_name` instead of `name` as the
image tag. The function signature changes from `build(image_tag: &str)` — callers pass
`kitchen.container_name()`.

---

## `src/container.rs` (new file)

Responsible for creating and starting a container. Accepts a shared `Docker` client and a
`&Kitchen` reference so it needs no extra dependencies.

After starting the container, stream its stdout/stderr and print each line until the sentinel
`"Kitchen is ready to cook"` appears, then return. This gives the caller a synchronisation point
before the container is used.

```rust
use bollard::Docker;
use bollard::container::LogOutput;
use bollard::models::{ContainerCreateBody, HostConfig};
use bollard::query_parameters::{CreateContainerOptionsBuilder, LogsOptionsBuilder};
use futures_util::StreamExt;

use crate::kitchen::Kitchen;

const READY_SENTINEL: &str = "Kitchen is ready to cook";

pub async fn run(docker: &Docker, kitchen: &Kitchen) -> Result<(), bollard::errors::Error> {
    let container_name = kitchen.container_name();

    let options = CreateContainerOptionsBuilder::default()
        .name(&container_name)
        .build();

    let body = ContainerCreateBody {
        image: Some(container_name.clone()),
        hostname: Some(container_name.clone()),
        env: Some(vec![kitchen.kitchen_workspace_env()]),
        host_config: Some(HostConfig {
            binds: Some(vec![kitchen.workspace_mount()]),
            ..Default::default()
        }),
        ..Default::default()
    };

    docker.create_container(Some(options), body).await?;
    docker.start_container(&container_name, None).await?;

    let log_options = LogsOptionsBuilder::default()
        .follow(true)
        .stdout(true)
        .stderr(true)
        .build();

    let mut stream = docker.logs(&container_name, Some(log_options));

    while let Some(result) = stream.next().await {
        match result {
            Ok(output) => {
                let line = output.to_string();
                print!("{line}");
                if line.contains(READY_SENTINEL) {
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
```

Return `Result` so the caller can decide how to handle errors (e.g. image not found, port
conflict).

---

## `src/main.rs` changes

1. Add `mod container;` alongside `mod image;`.

2. Update the `image::build` call in both `build` and `up` to pass `kitchen.container_name()`
   instead of `kitchen.name`.

3. In `up`, after `image::build`, call:
   ```rust
   container::run(&docker, &kitchen).await.expect("failed to start container");
   ```

4. The existing `inspect_container` guard (already present) handles the "already running" /
   "exists but stopped" cases before reaching `build` and `run` — extend the stopped-container
   branch to restart or remove-and-recreate as a follow-up.

---

## Bollard types reference

| Docker flag | Bollard type/field |
|---|---|
| `--name` | `CreateContainerOptions { name: &str }` |
| `--hostname` | `Config { hostname: Option<String> }` |
| `-e KEY=VALUE` | `Config { env: Option<Vec<String>> }` |
| `-v host:container` | `HostConfig { binds: Option<Vec<String>> }` |
| image positional | `Config { image: Option<String> }` |
| `-d` (detached) | implicit — `start_container` never attaches |
| log streaming | `docker.logs()` → `Stream<Item = Result<LogOutput, _>>` |
| `LogOutput::to_string()` | `Display` impl decodes the bytes as UTF-8 |

---

## Steps in order

1. Add `workspace_mount` and `kitchen_workspace_env` to `Kitchen`.
2. Update `image::build` call sites to use `kitchen.container_name()`.
3. Create `src/container.rs` with the `run` function.
4. Add `mod container;` to `main.rs` and wire up the call in `up`.
5. Smoke-test with a real workspace directory.
