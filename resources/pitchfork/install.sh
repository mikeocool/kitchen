#!/bin/bash

# TODO make version (and/or download URL) configurable

curl -fsSL -o /tmp/pitchfork.tar.gz https://github.com/endevco/pitchfork/releases/latest/download/pitchfork-x86_64-unknown-linux-gnu.tar.gz \
    && tar zxvf /tmp/pitchfork.tar.gz -C /tmp \
    && mv /tmp/pitchfork /usr/local/bin/pitchfork \
    && chmod 0755 /usr/local/bin/pitchfork
