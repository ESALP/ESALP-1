#!/bin/sh

# Bail on error
set -e

# Check formatting
cargo fmt -- --write-mode=diff

# Build rust
make
