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
- Look at git worktreess -- ideally the worktrees all end up on the host mount
- investigate tailscale split dns
    - run dnsmasq
    - setup tailscale split dns rule to route \*.<kitchen name>.ktchn.wtf to route to your kitchen dnsmasq
    - probably wont be able to get valid SSL certs
    - could also use a cloud dns provider, after tailscale up, it registers it's ip with the cloud dns provider
        - no split DNS needed as long as you BYO domain
        - could potenitally get ssl certs via DNS verification

- implement `kitchen ssh` and `kitchen shell`
    - possible to dynamically create a socket or network connection to send messages back to the kitchen client
    - ssh -R /tmp/my.sock:localhost:9000 user@remote - generate random socket, set env var on connection
    - allows for xdg-open to use $BROWSER and potentially git credential helper
    - kitchen ssh maybe works remotely using local tailscale API to find ktichens
- Option to clone a repo on to a volume instead of mounting from host
- Look at code tunnels and/or openvscode-server for editting via vscode.dev or running
