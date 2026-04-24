FROM debian:13-slim

# from https://mise.jdx.dev/mise-cookbook/docker.html

RUN apt-get update  \
    && apt-get -y --no-install-recommends install  \
        sudo curl git ca-certificates build-essential jq zsh \
    && rm -rf /var/lib/apt/lists/*

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# TODO these dont work for user
ENV MISE_DATA_DIR="/mise"
ENV MISE_CONFIG_DIR="/mise"
ENV MISE_CACHE_DIR="/mise/cache"
ENV MISE_INSTALL_PATH="/usr/local/bin/mise"
ENV PATH="/mise/shims:$PATH"

# Install mise
RUN curl https://mise.run | sh

# Install tailscale
RUN curl -fsSL https://tailscale.com/install.sh | sh

# Install docker
RUN curl -fsSL https://get.docker.com | sh

RUN useradd -m -s /bin/zsh k \
    && usermod -aG sudo,docker k \
    && echo "k ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/k
ENV KITCHEN_USER="k"

COPY init.sh /init.sh
RUN chmod +x init.sh
ENTRYPOINT ["/init.sh"]
# TODO add mise files and install their deps
