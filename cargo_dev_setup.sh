#!/bin/sh
echo "Starting Development Environment Setup"
cargo install grcov
rustup component add llvm-tools-preview