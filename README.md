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

- fixup tailscale exension
- docker outside of docker feature (figure out contributing mounts)
- custom additional mounts
- status command
- extensions can contribute context to up message (tailscale shares IP)
- label containers, images, volumes
    - generic command to list running kitchens
- down command should remove tailscale machine
- add local mount to preserve claude context
- extenion to add ghostty term-info
- Define toml format and ready from and merge:
    - <workspace>/.kitchen/config.toml
    - <workspace>/.kitchen/config.local.toml
    - <workspace>/.kitchen.toml(?)
