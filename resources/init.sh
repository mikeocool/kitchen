#!/bin/bash

set -euo pipefail

echo "Setting up Docker outside of Docker..."
DOCKER_SOCK=/var/run/docker.sock
if [ -S "$DOCKER_SOCK" ]; then
    DOCKER_SOCK_GID=$(stat -c '%g' "$DOCKER_SOCK")
    # Create a 'docker-host' group with the host's GID (if it doesn't exist)
    if ! getent group "$DOCKER_SOCK_GID" > /dev/null 2>&1; then
        sudo groupadd -g "$DOCKER_SOCK_GID" docker-host
    fi
    # add user to group
    sudo usermod -aG "$DOCKER_SOCK_GID" "$(whoami)" 2>/dev/null || true
else
    echo "WARNING: Docker socket not found at $DOCKER_SOCK" >&2
    echo "Did you mount it with -v /var/run/docker.sock:/var/run/docker.sock?" >&2
fi


# Start tailscaled
# echo "Starting tailscaled"
# sudo bash -c 'tailscaled --tun=userspace-networking --socks5-server=localhost:1055 >> /var/log/tailscaled.log 2>&1' &

# TODO better healthcheck for tailscaled
# echo "Waiting for tailscaled to start..."
# sleep 5

# TODO maybe also support qrcode?
# Piping to bash wont work here, since the process doesnt exit until auth is complete
#TAILSCALE_AUTH_URL=$(tailscale up --ssh --json | jq .AuthUrl)
# echo "Running tailscale up..."
# sudo tailscale up --ssh

# TAILSCALE_IP=$(tailscale ip --4)

/usr/local/bin/kitchen container-provision

echo "---"
# echo "Connect via ssh: ssh k@${TAILSCALE_IP}"
echo "Kitchen is ready to cook"

echo "Starting pitchfork..."
exec sudo pitchfork supervisor run --boot --container
