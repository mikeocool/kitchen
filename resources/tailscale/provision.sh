#!/usr/bin/env sh
set -e

# TODO fetch this script as part of container build and bake it into the container
curl -fsSL https://tailscale.com/install.sh | sh
