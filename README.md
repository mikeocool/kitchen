An alternative to devcontainers built on mise and tailscale.

```
./kitchen <path to workspace>
```

## TODO

Fix docker group

for ghostty to work well, need to run this:
infocmp -x xterm-ghostty | ssh k@<kitchen ip> -- tic -x -
