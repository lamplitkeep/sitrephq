#!/usr/bin/env bash
set -euo pipefail
cargo test --workspace
node --test tests/