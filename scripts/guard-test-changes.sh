#!/usr/bin/env bash
# Fail when tests/ (including tests/fixtures/) changes without a Test-Change: trailer.
#
# Usage:
#   guard-test-changes.sh                  # commits after merge-base / guard intro
#   guard-test-changes.sh --range FROM TO  # TO is exclusive in git's FROM..TO
#   guard-test-changes.sh --staged --message FILE
#
# FROM defaults to GUARD_BASE, else the PR/push base, else origin/main.
set -euo pipefail

trailer_ok() {
  git interpret-trailers --parse --no-divider | grep -q '^Test-Change:'
}

commit_has_trailer() {
  git log -1 --format=%B "$1" | trailer_ok
}

die() {
  echo "error: $*" >&2
  exit 1
}

mode=range
range_from=""
range_to="HEAD"
message_file=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --staged)
      mode=staged
      shift
      ;;
    --range)
      mode=range
      range_from=${2:?--range FROM TO}
      range_to=${3:?--range FROM TO}
      shift 3
      ;;
    --message)
      message_file=${2:?--message FILE}
      shift 2
      ;;
    --help|-h)
      sed -n '2,12p' "$0"
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

if [[ $mode == staged ]]; then
  changed=$(git diff --cached --name-only -- tests || true)
  if [[ -z $changed ]]; then
    echo "ok: no staged tests/ changes"
    exit 0
  fi
  if [[ -z $message_file ]]; then
    die "staged tests/ changes require --message FILE (commit message)"
  fi
  if trailer_ok <"$message_file"; then
    echo "ok: staged tests/ changes declared with Test-Change:"
    echo "$changed"
    exit 0
  fi
  echo "tests/ changed without a Test-Change: commit trailer:" >&2
  echo "$changed" >&2
  echo "add a trailer such as: Test-Change: why the tests moved" >&2
  exit 1
fi

if [[ -z $range_from ]]; then
  if [[ -n ${GUARD_BASE:-} ]]; then
    range_from=$GUARD_BASE
  elif git rev-parse --verify --quiet origin/main >/dev/null; then
    range_from=$(git merge-base HEAD origin/main)
  elif git rev-parse --verify --quiet main >/dev/null; then
    range_from=$(git merge-base HEAD main)
  else
    die "cannot find merge base; set GUARD_BASE"
  fi
fi

from_commit=$(git rev-parse --verify "$range_from")
to_commit=$(git rev-parse --verify "$range_to")

# Once this workflow exists on the base, every PR commit is in scope.
# While it is landing, only commits after the introducing commit are new.
workflow_rel=.github/workflows/guard-test-changes.yml
if git cat-file -e "${from_commit}:${workflow_rel}" 2>/dev/null; then
  start=$from_commit
else
  introduced=$(git log --reverse --format=%H "${from_commit}..${to_commit}" -- "$workflow_rel")
  introduced=${introduced%%$'\n'*}
  if [[ -z $introduced ]]; then
    echo "ok: guard workflow not in ${from_commit}..${to_commit}; nothing to enforce yet"
    exit 0
  fi
  start=$introduced
fi

commits=$(git rev-list --reverse "${start}..${to_commit}")
if [[ -z $commits ]]; then
  echo "ok: no commits after ${start}"
  exit 0
fi

failed=0
while read -r commit; do
  [[ -z $commit ]] && continue
  changed=$(git diff-tree --no-commit-id --name-only -r "$commit" -- tests || true)
  if [[ -z $changed ]]; then
    continue
  fi
  if commit_has_trailer "$commit"; then
    continue
  fi
  echo "commit ${commit} changes tests/ without a Test-Change: trailer:" >&2
  echo "$changed" >&2
  failed=1
done <<<"$commits"

if [[ $failed -ne 0 ]]; then
  echo "add a trailer such as: Test-Change: why the tests moved" >&2
  exit 1
fi

echo "ok: tests/ changes after ${start} declare Test-Change:"
exit 0
