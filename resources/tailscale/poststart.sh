#!/usr/bin/env sh
set -e

# TODO check if tailscale is already running

# TODO maybe also support qrcode?
# Piping to bash wont work here, since the process doesnt exit until auth is complete
#TAILSCALE_AUTH_URL=$(tailscale up --ssh --json | jq .AuthUrl)
echo "Running tailscale up..."
sudo tailscale up --ssh
