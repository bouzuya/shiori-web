#!/bin/bash

curl -fsSL https://claude.ai/install.sh | bash

rustup component add --toolchain nightly rustfmt
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
