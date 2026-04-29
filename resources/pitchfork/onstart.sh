#!/usr/bin/env sh
set -e

# needs to run a sudo

mkdir -p /etc/pitchfork/
echo "[settings.supervisor]\nuser = \"k\"\n\n" > /etc/pitchfork/config.toml
cat /etc/kitchen/daemons/*.toml >> /etc/pitchfork/config.toml
chown k:k /etc/pitchfork/config.toml
