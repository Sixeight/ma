#!/usr/bin/env bash
# Block git commit when tests/ is in the commit without a Test-Change: trailer.
set -euo pipefail

payload=$(cat)
command=$(python3 -c 'import json,sys; print(json.loads(sys.stdin.read()).get("command",""))' <<<"$payload")

allow() {
  printf '{"permission":"allow"}\n'
  exit 0
}

deny() {
  python3 -c 'import json,sys; print(json.dumps({"permission":"deny","user_message":sys.argv[1],"agent_message":sys.argv[1]}))' "$1"
  exit 0
}

if ! grep -Eq '(^|[[:space:];&|])git[[:space:]]+commit($|[[:space:]])' <<<"$command"; then
  allow
fi

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

will_stage_all=0
if grep -Eq 'git[[:space:]]+add[[:space:]]+(-A|--all|\.)' <<<"$command"; then
  will_stage_all=1
fi

changed=$(git diff --cached --name-only -- tests || true)
if [[ $will_stage_all -eq 1 ]]; then
  extra=$(
    {
      git diff --name-only -- tests
      git ls-files --others --exclude-standard -- tests
    } || true
  )
  changed=$(printf '%s\n%s\n' "$changed" "$extra" | sed '/^$/d' | sort -u)
fi

if [[ -z $changed ]]; then
  allow
fi

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
python3 - "$command" >"$tmp" <<'PY'
import re, sys
command = sys.argv[1]
parts = [a or b for a, b in re.findall(r"-m\s+(?:'([^']*)'|\"([^\"]*)\")", command)]
text = "\n\n".join(parts) if parts else command
sys.stdout.write(text)
PY

if git interpret-trailers --parse --no-divider <"$tmp" | grep -q '^Test-Change:'; then
  allow
fi

deny "tests/ changed without a Test-Change: commit trailer"
