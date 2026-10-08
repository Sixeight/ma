#!/usr/bin/env bash
# Compare renders of diagrams that contain no subgraphs against another ma binary.
# Usage: compare_no_subgraph.sh <baseline-ma> <candidate-ma>
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <baseline-ma> <candidate-ma>" >&2
  exit 2
fi

baseline=$1
candidate=$2
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

inputs=(
  $'graph TD\n    A --> B\n'
  $'graph TD\n    A[Start] --> B[End]\n'
  $'graph TD\n    A --> B\n    A --> C\n'
  $'graph TD\n    A --> C\n    B --> C\n'
  $'graph TD\n    A -->|yes| B\n    A -->|no| C\n'
  $'graph TD\n    A --> B\n    B --> C\n    C --> A\n'
  $'graph LR\n    A --> B\n    B --> C\n'
  $'graph LR\n    A[Start] --> B{Choice}\n    B -->|yes| C\n    B -->|no| D\n'
  $'flowchart TD\n    A --> B\n    B --> C\n    C --> A\n'
  $'graph TD\n    A --> B\n    B -->|fallback| B\n    B --> C\n'
)

failed=0
for i in "${!inputs[@]}"; do
  printf '%s' "${inputs[$i]}" > "$tmpdir/in.mmd"
  "$baseline" "$tmpdir/in.mmd" > "$tmpdir/base.txt"
  "$candidate" "$tmpdir/in.mmd" > "$tmpdir/cand.txt"
  if ! cmp -s "$tmpdir/base.txt" "$tmpdir/cand.txt"; then
    echo "MISMATCH diagram $i"
    diff -u "$tmpdir/base.txt" "$tmpdir/cand.txt" || true
    failed=1
  fi
done

if [[ $failed -eq 0 ]]; then
  echo "ok: ${#inputs[@]} no-subgraph diagrams are byte-identical"
fi
exit $failed
