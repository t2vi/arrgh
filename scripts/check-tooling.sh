#!/bin/sh
# Verifies the tooling this project's workflows need: every tool is defined in
# .claude/tooling-catalog.yaml, each workflow's chain in .claude/skills-registry.yaml, and the
# enabled workflows in .claude/profile.yaml (`skills:`). Global tools live in ~/.claude, not the
# project — nothing to install per-repo, this just confirms they're active before you start.
set -u
cd "$(dirname "$0")/.." || exit 1
catalog=.claude/tooling-catalog.yaml
registry=.claude/skills-registry.yaml

pass=0
fail=0

ok()   { printf '  \033[0;32m\342\234\224\033[0m %s\n' "$1"; pass=$((pass + 1)); }
bad()  { printf '  \033[0;31m\342\234\227\033[0m %s\n' "$1"; fail=$((fail + 1)); }
na()   { printf '  \033[0;90m-\033[0m %s\n' "$1"; }
note() { printf '  \033[0;90m-\033[0m %s (optional)\n' "$1"; }

# Line format is the catalog header's contract. One tab-separated record per entry, empty
# field = "-": name kind required provides relevant_when check install purpose
records="$([ -f "$catalog" ] && awk '
  function out() { if (n != "") { printf "%s", n; for (i = 1; i <= 7; i++) printf "\t%s", (K[i] in f && f[K[i]] != "" ? f[K[i]] : "-"); print "" } }
  BEGIN { split("kind required provides relevant_when check install purpose", K, " ") }
  /^#/ { next }
  /^[a-z0-9-]+:$/ { out(); n = substr($0, 1, length($0) - 1); split("", f); next }
  /^  [a-z_]+: / { k = $1; sub(/:$/, "", k); v = $0; sub(/^  [a-z_]+: /, "", v); f[k] = v }
  END { out() }' "$catalog")"
tab="$(printf '\t')"
# ponytail: each MCP check runs its own `claude mcp list`; cache it if the check gets slow
active() { sh -c "$1" </dev/null >/dev/null 2>&1; }
# the catalog entry named $1, or the one that provides skill $1
provider() {
  [ "$1" = project-checks ] && { echo project-checks; return; }
  printf '%s\n' "$records" | awk -F'\t' -v t="$1" '
    $1 == t { print $1; found = 1; exit }
    { n = split($4, a, / *, */); for (i = 1; i <= n; i++) if (a[i] == t && p == "") p = $1 }
    END { if (!found) print (p == "" ? "?" : p) }'
}

echo "Required (global):"
while IFS="$tab" read -r name _ req _ _ chk inst purp; do
  [ "$req" = global ] || continue
  if active "$chk"; then ok "$name: $purp"; else bad "$name not active — $inst"; fi
done <<EOF
$records
EOF

# Required steps of each enabled workflow's chain, grouped by the tool that provides them.
skills="$(sh scripts/profile-get.sh skills)"
for s in ${skills:-coding}; do
  if ! grep -q "^$s:" "$registry" 2>/dev/null; then
    echo "Skill $s:"; na "not in the skills registry — ignored"; continue
  fi
  groups="$(awk -v k="$s" '/^[^ #]/ { on = ($0 ~ "^" k ":") } on && /^    - / { sub(/^    - /, ""); print }' "$registry" |
    awk -F' [|] ' '$4 == "required" && !seen[$2]++ { print $2 }' |
    while read -r t; do echo "$(provider "$t") $t"; done |
    awk '!($1 in g) { o[++n] = $1 } { g[$1] = ($1 in g ? g[$1] ", " : "") $2 } END { for (i = 1; i <= n; i++) print o[i], g[o[i]] }')"
  [ -n "$groups" ] || continue
  echo "Required by $s:"
  while read -r p tools; do
    case "$p" in
      project-checks)
        rc="$(sh scripts/profile-get.sh required_checks | tr '\n' ',' | sed 's/,$//; s/,/, /g')"
        na "project-checks: ${rc:-none recorded (profile required_checks)}" ;;
      "?") bad "$tools: not in $catalog" ;;
      *)
        r="$(printf '%s\n' "$records" | awk -F'\t' -v n="$p" '$1 == n')"
        kind="$(echo "$r" | cut -f2)" chk="$(echo "$r" | cut -f6)" inst="$(echo "$r" | cut -f7)"
        if active "$chk"; then ok "$tools ($p)"
        elif [ "$kind" = component ]; then na "$tools ($p): n/a — component not in use"
        else bad "$tools ($p) not active — $inst"; fi ;;
    esac
  done <<EOF
$groups
EOF
done

echo "Optional (active, from $catalog):"
# a tool absent never fails
while IFS="$tab" read -r name kind req _ _ chk _ purp; do
  [ "$req" = - ] || continue
  case "$kind" in builtin|template-skill|component) continue ;; esac
  active "$chk" && note "$name: $purp"
done <<EOF
$records
EOF

echo
echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
