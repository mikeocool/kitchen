#!/usr/bin/env sh
set -e

REPO="$1"
INSTALL_CMD="$2"
DOTFILES_DIR="${HOME}/dotfiles"

echo "Installing dotfiles..."

if ! command -v git > /dev/null 2>&1; then
    echo "Error: git is not installed but dotfiles_repo is configured" >&2
    exit 1
fi

# TODO check if repo has expected origin?
if [ -d "$DOTFILES_DIR" ]; then
    echo "${DOTFILES_DIR} already exists, updating..."
    cd "$DOTFILES_DIR"
    BEFORE=$(git rev-parse HEAD)
    git pull
    AFTER=$(git rev-parse HEAD)
    if [ "$BEFORE" = "$AFTER" ]; then
        exit 0
    fi
else
    echo "${DOTFILES_DIR} does not exist, cloning repo..."
    git clone "$REPO" "$DOTFILES_DIR"
    cd "$DOTFILES_DIR"
fi

if [ -n "$INSTALL_CMD" ]; then
    sh -c "$INSTALL_CMD"
fi
