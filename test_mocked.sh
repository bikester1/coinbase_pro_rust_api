#!/bin/sh
export RUSTFLAGS="-Cinstrument-coverage -Ccodegen-units=4"
export LLVM_PROFILE_FILE="./Profiling/%p-%m.profraw"
export RUST_BACKTRACE=1

#cargo clean
cargo test --tests --features mock -- --nocapture
#cargo test --tests -- --nocapture
grcov . --binary-path "./target/debug" -s . -t html --branch --llvm -o ./coverage/
