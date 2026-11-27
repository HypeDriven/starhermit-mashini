#!/bin/sh
set -eu
cargo build --release --target wasm32-unknown-unknown --offline
cp target/wasm32-unknown-unknown/release/mashini.wasm .
engine=${STARHERMIT:-$HOME/sh/runtime/1.53.0/starhermit-1.53.0-linux-x86_64/starhermit}
"$engine" package --web .
"$engine" check .
