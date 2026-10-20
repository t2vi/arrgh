#!/bin/sh
# Prints one value from .claude/profile.yaml (or $PROFILE), else the default. Always exits 0.
#
#   scripts/profile-get.sh branching gitflow            # top-level scalar
#   scripts/profile-get.sh components.merge-guard true  # entry of a top-level map
#   scripts/profile-get.sh required_checks              # list: one item per line
#
# Line-oriented on purpose — no YAML parser (specs/016-project-wizard/contracts/profile-schema.md):
# top-level keys at column 0, list items `  - item`, map entries indented two spaces as
# `  name: value`, one value per line, `#` comments allowed.
set -u

key="${1:?usage: profile-get.sh <key|map.name> [default]}"
default="${2:-}"
file="${PROFILE:-$(git rev-parse --show-toplevel 2>/dev/null)/.claude/profile.yaml}"

val=""
if [ -f "$file" ]; then
  val="$(awk -v key="$key" '
    function clean(s) { sub(/[ \t]+#.*$/, "", s); sub(/[ \t]+$/, "", s); return s }
    BEGIN { n = index(key, "."); if (n) { map = substr(key, 1, n - 1); name = substr(key, n + 1) } }
    /^[ \t]*#/ { next }
    !n && index($0, key ": ") == 1 { print clean(substr($0, length(key) + 3)); exit }
    !n && /^[^ \t]/ { inlist = ($0 ~ "^" key ":[ \t]*(#.*)?$"); next }
    !n && inlist && /^  - / { print clean(substr($0, 5)) }
    n && /^[^ \t]/ { inmap = ($0 ~ "^" map ":[ \t]*(#.*)?$") ; next }
    n && inmap && index($0, "  " name ": ") == 1 { print clean(substr($0, length(name) + 5)); exit }
  ' "$file")"
fi
printf '%s\n' "${val:-$default}"
