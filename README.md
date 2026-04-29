An alternative to devcontainers built on mise and tailscale.

```
./kitchen <path to workspace>
```

## Development

Create dev kitchen

Need to bootstrap a `cargo build`

```
./target/debug/kitchen up
```

SSH in

```
cd /workspaces/kitchen
cargo run
```

## TODO

- save tailscale state
- mise shims
- Define toml format and ready from and merge:
    - <workspace>/.kitchen/config.toml
    - <workspace>/.kitchen/config.local.toml
    - <workspace>/.kitchen.toml(?)
