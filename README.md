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

- eyre/anyhow
- support relative mount paths?
- status command/centralizing inspecting running container
- mise extension
- docker outside of docker extension (figure out contributing mounts)
- extensions can contribute context to up message (tailscale shares IP)
- installing system packages
- install hook for extensions + contributing to docker image
- when tailsacle ssh needs re-auth, zed ssh connection just hangs
- label containers, images, volumes
    - generic command to list running kitchens
- down command should remove tailscale machine
- add local mount to preserve claude context
- extenion to add ghostty term-info
- Define toml format and ready from and merge:
    - <workspace>/.kitchen/config.toml
    - <workspace>/.kitchen/config.local.toml
    - <workspace>/.kitchen.toml(?)
- Look at git worktreess -- ideally the worktrees all end up on the hostmount
