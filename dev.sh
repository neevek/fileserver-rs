#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

(trap 'kill 0' SIGINT; \
 # bash -c 'cd frontend; CARGO_TARGET_DIR=../target-trunk trunk serve --address 0.0.0.0' & \
 # bash -c 'cd backend; cargo watch -- cargo run -- --port 8081')

 bash -c 'cd frontend; trunk serve --address 0.0.0.0 --port 8082 --proxy-backend=http://[::1]:8081/api/' & \
 bash -c 'cargo watch -- cargo run --bin backend -- --port 8081')
