An alternative to devcontainers built on mise and tailscale.

```
./kitchen <path to workspace>
```

## Development

Create dev kitchen

```
./poc/kitchen
```

SSH in

```
cd /workspaces/kitchen
cargo run
```

## TODO

- Implement run
- Define toml format and ready from and merge:
    - <workspace>/.kitchen/config.toml
    - <workspace>/.kitchen/config.local.toml
    - <workspace>/.kitchen.toml(?)
