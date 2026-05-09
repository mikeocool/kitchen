# Plan: `ScriptRunner::from_url` — fetch and run remote scripts

## Goal

Support the common `curl https://example.com/install.sh | sh` installation pattern by fetching a script from a URL and returning a `ScriptRunner` ready to chain and run. The caller retains full control over sudo, shell, label, timeout, etc. after the fetch.

---

## New dependency: `reqwest`

No HTTP client is currently in the tree. `reqwest` is the standard choice — async, tokio-native, and supports HTTPS out of the box. Use `rustls-tls` to avoid an OpenSSL system dependency.

```toml
# Cargo.toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
```

---

## API

`from_url` is an async constructor on `ScriptRunner` that fetches the script and returns `Self`, preserving the full builder chain:

```rust
// Fetch, then run with any builder options:
ScriptRunner::from_url("https://mise.run")
    .await?
    .label("mise installer")
    .sudo()
    .run()
    .await?;

// Convenience free function for the simple case:
pub async fn run_url(url: &str) -> Result<()> {
    ScriptRunner::from_url(url).await?.run().await
}
```

---

## Implementation

```rust
impl ScriptRunner {
    pub async fn from_url(url: impl AsRef<str>) -> Result<Self> {
        let url = url.as_ref();

        // Reject plain HTTP — remote scripts must come over TLS.
        if !url.starts_with("https://") {
            return Err(eyre!("refusing to fetch script over plain HTTP: {url}"));
        }

        let response = reqwest::get(url)
            .await
            .wrap_err_with(|| format!("failed to fetch {url}"))?;

        if !response.status().is_success() {
            return Err(eyre!(
                "fetching {url} returned status {}",
                response.status()
            ));
        }

        let script = response
            .text()
            .await
            .wrap_err_with(|| format!("failed to read response body from {url}"))?;

        Ok(Self::new(ScriptInput::Script(script)))
    }
}
```

`wrap_err_with` (from `eyre::WrapErr`) adds the URL to the error context so failures are self-describing without requiring the caller to do it.

---

## Security considerations

The `curl | sh` pattern has real risks — a compromised CDN, a redirect to a different host, or a typo in the URL can run arbitrary code. This plan addresses the obvious ones and defers the rest.

### HTTPS only (enforced in v1)

Plain `http://` URLs are rejected at the call site before any network I/O. `reqwest` with `rustls-tls` verifies the server certificate by default. No opt-out — if a script is only available over HTTP, the caller must fetch it themselves and use `ScriptRunner::script()`.

### Preview before run (v1 — opt-in)

A `preview()` builder method prints the full script to stdout before executing. Useful when running interactively and the user wants to inspect what they're about to run.

```rust
pub fn preview(mut self) -> Self {
    self.preview = true;
    self
}

// In execute(), before spawning the child:
if self.preview {
    println!("--- script from {url} ---");
    println!("{script}");
    println!("--- end ---");
}
```

This requires storing the source URL on the struct (for display). Add `source_url: Option<String>` to `ScriptRunner` and populate it in `from_url`.

### Checksum verification (defer to v2)

Allow the caller to supply an expected SHA-256 digest. If the downloaded content doesn't match, return an error before running anything.

```rust
// v2 API sketch:
ScriptRunner::from_url("https://example.com/install.sh")
    .await?
    .verify_sha256("e3b0c44298fc1c149afb...")
    .run()
    .await?;
```

Deferred because: many install scripts don't publish checksums, and the HTTPS guarantee already covers the transport layer for most cases.

### Redirect following

`reqwest` follows redirects by default (up to 10). This is the right default — many install script URLs intentionally redirect (e.g. `mise.run` → `mise.jdx.dev/install.sh`). The final URL after redirects is what gets fetched; the TLS check applies to every hop.

---

## Struct changes

Add two fields to `ScriptRunner` to support preview display:

```rust
pub struct ScriptRunner {
    input: ScriptInput,
    sudo: bool,
    shell: String,
    working_dir: Option<PathBuf>,
    env: HashMap<String, String>,
    timeout: Option<Duration>,
    label: Option<String>,
    preview: bool,          // print script content before running
    source_url: Option<String>, // set by from_url, shown in preview header
}
```

Both default to `false` / `None` in `ScriptRunner::new()`.

---

## Files to create / modify

| File             | Action                                                                 |
| ---------------- | ---------------------------------------------------------------------- |
| `Cargo.toml`     | Add `reqwest` with `rustls-tls`                                        |
| `src/cmd/mod.rs` | Add `from_url`, `run_url`, `preview` builder, `source_url` struct field |
