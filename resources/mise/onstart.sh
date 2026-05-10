#!/usr/bin/env bash
set -e

# TODO support other shells
# Shims for non-interaative sessions
echo 'eval "$(mise activate zsh --shims)"' >> ~/.zprofile
# activate for interactive sessions
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

echo "kw: ${KITCHEN_WORKSPACE}"
cd ${KITCHEN_WORKSPACE}
mise trust --all
mise install
