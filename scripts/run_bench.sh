#!/usr/bin/env bash
# Reproducible in-process render bench. Release binary, no CLI IO in the timed region.
set -euo pipefail
repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"
cargo build --release --bins --benches
python3 scripts/bench_inputs.py benches/inputs
echo "load average: $(uptime)"
echo "nproc=$(nproc)"
echo "rustc=$(rustc --version)"
cargo bench --bench render -- --dir benches/inputs "$@"
