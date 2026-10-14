#!/usr/bin/env bash
# Compare rendered output of two ma binaries over fixture and generated diagrams.
# Usage: compare_renders.sh <baseline-ma> <candidate-ma> [inputs-dir]
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "usage: $0 <baseline-ma> <candidate-ma> [inputs-dir]" >&2
  exit 2
fi

baseline=$1
candidate=$2
repo_root=$(cd "$(dirname "$0")/.." && pwd)
inputs=${3:-"$repo_root/benches/inputs"}

if [[ ! -d $inputs/generated ]]; then
  python3 "$repo_root/scripts/bench_inputs.py" "$inputs"
fi

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

failed=0
compared=0
while IFS= read -r -d '' file; do
  rel=${file#"$inputs"/}
  "$baseline" "$file" >"$tmpdir/base.txt"
  "$candidate" "$file" >"$tmpdir/cand.txt"
  compared=$((compared + 1))
  if ! cmp -s "$tmpdir/base.txt" "$tmpdir/cand.txt"; then
    echo "MISMATCH $rel"
    diff -u "$tmpdir/base.txt" "$tmpdir/cand.txt" | sed -n '1,80p' || true
    failed=1
  fi
done < <(find "$inputs/fixtures" "$inputs/generated" -name '*.mmd' -print0 | sort -z)

if [[ $failed -eq 0 ]]; then
  echo "ok: $compared diagrams are byte-identical"
fi
exit $failed
