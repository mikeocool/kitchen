#!/bin/bash

set -euo pipefail

if [[ -f "${KITCHEN_WORKSPACE}/.kitchen/.env" ]]; then
    source "${KITCHEN_WORKSPACE}/.kitchen/.env"
fi

if [[ -f "${KITCHEN_WORKSPACE}/.kitchenenv" ]]; then
    source "${KITCHEN_WORKSPACE}/.kitchenenv"
fi

if [[ -f "${KITCHEN_WORKSPACE}/.kitchen/.env.local" ]]; then
    source "${KITCHEN_WORKSPACE}/.kitchen/.env.local"
fi

if [[ -f "${KITCHEN_WORKSPACE}/.kitchenenv.local" ]]; then
    source "${KITCHEN_WORKSPACE}/.kitchenenv.local"
fi

# Remap docker group GID to match the host socket so all sessions (including SSH) can use Docker
if [[ -n "${DOCKER_SOCK_GID:-}" ]]; then
    sudo groupmod -g "${DOCKER_SOCK_GID}" docker
fi

echo "Setting up Docker outside of Docker..."
DOCKER_SOCK=/var/run/docker.sock
if [ -S "$DOCKER_SOCK" ]; then
    DOCKER_SOCK_GID=$(stat -c '%g' "$DOCKER_SOCK")
    # Create a 'docker' group with the host's GID (if it doesn't exist)
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
echo "Starting tailscaled"
sudo bash -c 'tailscaled --tun=userspace-networking --socks5-server=localhost:1055 >> /var/log/tailscaled.log 2>&1' &

# TODO better healthcheck for tailscaled
echo "Waiting for tailscaled to start..."
sleep 5

# TODO maybe also support qrcode?
# Piping to bash wont work here, since the process doesnt exit until auth is complete
#TAILSCALE_AUTH_URL=$(tailscale up --ssh --json | jq .AuthUrl)
echo "Running tailscale up..."
sudo tailscale up --ssh

TAILSCALE_IP=$(tailscale ip --4)

if [[ -n "${KITCHEN_DOTFILES_REPO:-}" ]]; then
    echo "Cloning dotfiles from ${KITCHEN_DOTFILES_REPO}..."
    git clone "${KITCHEN_DOTFILES_REPO}" "${HOME}/dotfiles"

    if [[ -n "${KITCHEN_DOTFILES_INSTALL_CMD:-}" ]]; then
        echo "Running dotfiles install command..."
        cd "${HOME}/dotfiles" && "${KITCHEN_DOTFILES_INSTALL_CMD}"
    fi
fi

echo "Setting up mise"

# TODO support other shells
echo 'eval "$(/usr/local/bin/mise activate zsh)"' >> ~/.zshrc

if [[ -f "${KITCHEN_WORKSPACE}/.kitchen/mise.global.toml" ]]; then
    if [[ -f "${HOME}/.config/mise/config.toml" ]]; then
        echo "ERROR: ${HOME}/.config/mise/config.toml already exists; cannot link mise global config"
        exit 1
    else
        mkdir -p "${HOME}/.config/mise"
        ln -s "${KITCHEN_WORKSPACE}/.kitchen/mise.global.toml" "${HOME}/.config/mise/config.toml"
    fi
fi

cd ${KITCHEN_WORKSPACE}
mise trust --all
mise install
echo "---"
echo "Connect via ssh: ssh k@${TAILSCALE_IP}"
echo "Kitchen is ready to cook"


sleep infinity
