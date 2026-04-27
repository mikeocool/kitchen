FROM debian:13-slim

# from https://mise.jdx.dev/mise-cookbook/docker.html

RUN apt-get update  \
    && apt-get -y --no-install-recommends install  \
        sudo curl git ca-certificates build-essential jq zsh vim openssh-client pkg-config libssl-dev locales \
    && rm -rf /var/lib/apt/lists/*

# fixes terminal wonkiness (but will presumably need to be different for non-us users)
# maybe do the locale-get in the init script with the user's LANG?
RUN echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen && locale-gen

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV MISE_INSTALL_PATH="/usr/local/bin/mise"
#ENV PATH="/mise/shims:$PATH" #TODO?

# Install mise
RUN curl https://mise.run | sh

# Install tailscale
RUN curl -fsSL https://tailscale.com/install.sh | sh

# Install docker
RUN curl -fsSL https://get.docker.com | sh

RUN useradd -m -s /bin/zsh k \
    && usermod -aG sudo k \
    && echo "k ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/k


COPY init.sh /init.sh
RUN chmod +x init.sh

USER k
ENTRYPOINT ["/init.sh"]
# TODO add mise files and install their deps
