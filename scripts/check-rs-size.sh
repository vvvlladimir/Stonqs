#!/usr/bin/env bash
# Warns about a Rust source file that has outgrown one subject (CLAUDE.md: such a module becomes a
# folder with `mod.rs`). Counts code lines only — blank lines, `//` comments and an inline
# `#[cfg(test)] mod … { }` block are free — and skips test files (`tests.rs`, `tests/`) entirely,
# so documentation and tests never push a file over.
#
#   scripts/check-rs-size.sh            warn above WARN, fail above FAIL
#   WARN=400 scripts/check-rs-size.sh   override a limit
#
# Under GitHub Actions a warning is an annotation on the file, visible on the pull request.
set -euo pipefail

WARN=${WARN:-500}
FAIL=${FAIL:-800}

cd "$(git rev-parse --show-toplevel)"

status=0
while IFS= read -r file; do
  # Listed but deleted in the working tree: nothing to measure.
  [[ -f "$file" ]] || continue
  lines=$(awk '
    # An inline test module is the tail of a file by convention: stop counting there. A bare
    # `#[cfg(test)] mod tests;` declares a separate file and is not the start of a block.
    /^[[:space:]]*#\[cfg\(test\)\]/ { pending = 1; next }
    pending && /^[[:space:]]*(pub(\([a-z]+\))? )?mod [a-z_0-9]+[[:space:]]*\{/ { exit }
    { pending = 0 }
    /^[[:space:]]*$/ { next }
    /^[[:space:]]*\/\// { next }
    { n++ }
    END { print n + 0 }
  ' "$file")

  if (( lines > FAIL )); then
    msg="$file has $lines code lines (limit $FAIL): split it into a folder of modules"
    [[ -n "${GITHUB_ACTIONS:-}" ]] && echo "::error file=$file::$msg" || echo "error: $msg"
    status=1
  elif (( lines > WARN )); then
    msg="$file has $lines code lines (over $WARN): consider splitting it before adding more"
    [[ -n "${GITHUB_ACTIONS:-}" ]] && echo "::warning file=$file::$msg" || echo "warning: $msg"
  fi
done < <(git ls-files --cached --others --exclude-standard '*.rs' | sort -u | grep -Ev '(^|/)tests(/|\.rs$)')

exit $status
