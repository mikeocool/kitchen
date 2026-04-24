#!/bin/bash

set -euo pipefail

# Start tailscaled
echo "Starting tailscaled"
tailscaled --tun=userspace-networking --socks5-server=localhost:1055 >> /var/log/tailscaled.log 2>&1 &

# TODO better healthcheck for tailscaled
echo "Waiting for tailscaled to start..."
sleep 5

# TODO maybe also support qrcode?
# Piping to bash wont work here, since the process doesnt exit until auth is complete
#TAILSCALE_AUTH_URL=$(tailscale up --ssh --json | jq .AuthUrl)
echo "Running tailscale up..."
tailscale up --ssh

TAILSCALE_IP=$(tailscale ip --4)
echo "Connect via ssh: k@${TAILSCALE_IP}"

# TODO install dotfiles based on env

echo "Kitchen is ready to cook"


sleep infinity
