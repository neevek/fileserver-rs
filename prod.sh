#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

# (trap 'kill 0' SIGINT; \
 # bash -c 'cd frontend; CARGO_TARGET_DIR=../target-trunk trunk serve --address 0.0.0.0' & \
 # bash -c 'cd backend; cargo watch -- cargo run -- --port 8081')

 pushd frontend
 trunk build --release --public-url '/assets'
 popd

 mkdir -p testdir
 cargo run --bin backend -- --port 8082 --serve-dir ./testdir --assets-dir ./frontend/dist
