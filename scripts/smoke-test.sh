#!/bin/sh
# Smoke test for the template's shell scripts, run in a throwaway clone so the
# real working tree is never touched. Used by .github/workflows/scripts.yml;
# safe to run locally too: scripts/smoke-test.sh
# Runs in OAIKit and in every derived project: it checks only the template's own scripts
# (TEMPLATE_SCRIPTS — scripts.yml lints the same list) and only the components the project's
# profile uses; OAIKit-only assertions run only where docs/maintaining.md exists.
set -eu

SRC="$(git rev-parse --show-toplevel)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
export GIT_AUTHOR_NAME=smoke GIT_AUTHOR_EMAIL=smoke@example.invalid
export GIT_COMMITTER_NAME=smoke GIT_COMMITTER_EMAIL=smoke@example.invalid

fail() { echo "FAIL: $1" >&2; exit 1; }
pass() { echo "ok - $1"; }

# spec: 012/FR-003
git clone -q "$SRC" "$TMP/repo"
cd "$TMP/repo"
cp -p "$SRC"/scripts/*.sh scripts/   # test the working tree's scripts, not just what's committed
# ponytail: gitignored dirs (private overlay) are absent from the clone — create them first
mkdir -p .claude docs/workflows docs/releases .github/workflows
for f in .claude/tooling-catalog.yaml .claude/skills-registry.yaml .claude/profile.yaml .claude/settings.json CHANGELOG.md VERSION; do
  if [ -f "$SRC/$f" ]; then cp "$SRC/$f" "$f"; fi
done
rm -f .github/workflows/*.yml && cp "$SRC"/.github/workflows/*.yml .github/workflows/ 2>/dev/null || true
if [ -d "$SRC/docs/workflows" ]; then cp "$SRC"/docs/workflows/*.md docs/workflows/; fi
if [ -d "$SRC/docs/releases" ]; then cp "$SRC"/docs/releases/*.md docs/releases/; fi
# gitignored living docs never reach the clone — copy them in so backup/overlay have real input
for f in CLAUDE.md CONTEXT.md; do
  if [ -f "$SRC/$f" ] && [ ! -f "$f" ]; then cp "$SRC/$f" "$f"; fi
done

TEMPLATE_SCRIPTS="backup-private-docs.sh private-overlay.sh repo-setup.sh check-tooling.sh profile-get.sh smoke-test.sh"
OAIKIT=""; if [ -f docs/maintaining.md ]; then OAIKIT=1; fi
uses() { [ "$(sh scripts/profile-get.sh "components.$1" true)" != false ]; }

# spec: 012/FR-004
for s in $TEMPLATE_SCRIPTS; do
  if [ -f "scripts/$s" ]; then sh -n "scripts/$s" || fail "syntax: scripts/$s"; fi
done
# spec: 012/FR-005
for s in $TEMPLATE_SCRIPTS; do
  if [ -f "scripts/$s" ]; then [ -x "scripts/$s" ] || fail "scripts/$s isn't executable"; fi
done
pass "all template scripts parse and are executable"

# --- backup-private-docs.sh -------------------------------------------------
# spec: 005/FR-001, 005/FR-002
BK="$TMP/backup.git"
mkdir -p .claude/worktrees/junk && echo x > .claude/worktrees/junk/f   # machine-local: never backed up
sh scripts/backup-private-docs.sh "$BK" >/dev/null
files="$(git --git-dir="$BK" ls-tree -r --name-only HEAD 2>/dev/null || true)"
for f in CLAUDE.md .claude/profile.yaml .claude/private-overlay.conf .claude/skills/spec-audit/SKILL.md .claude/tooling-catalog.yaml; do
  if [ -e "$f" ]; then echo "$files" | grep -qx "$f" || fail "backup missing $f"; fi
done
echo "$files" | grep -q '^.claude/worktrees/' && fail "backup included .claude/worktrees"
rm -rf .claude/worktrees
pass "backup snapshots living docs, skips machine-local junk"
# spec: 005/FR-003
[ -z "$(git --git-dir="$BK" remote)" ] || fail "backup repo has a remote"
pass "backup repo has no remote"
# spec: 005/FR-006
if [ -f .claude/private-overlay.conf ]; then
  bpaths="$(sed -n '/^PATHS="/,/^"/p' scripts/backup-private-docs.sh)"
  for x in $(sh -c '. .claude/private-overlay.conf; echo $PRIVATE_PATHS'); do
    echo "$bpaths" | grep -qx "$x" || fail "backup PATHS missing overlay path $x"
  done
  pass "backup paths cover every overlay path"
fi
# spec: 005/FR-004
sh scripts/backup-private-docs.sh "$BK" | grep -q "No changes" || fail "backup not idempotent"
pass "backup is idempotent"

# --- private-overlay.sh (only if the project uses it) -----------------------
if [ -f .claude/private-overlay.conf ] && uses private-overlay; then
# spec: 006/FR-001, 006/FR-009
mv .claude/private-overlay.conf "$TMP/conf.bak"
sh scripts/private-overlay.sh status >/dev/null 2>&1 && fail "overlay ran without a conf"
grep -v '^PRIVATE_PATHS=' "$TMP/conf.bak" > .claude/private-overlay.conf
sh scripts/private-overlay.sh status >/dev/null 2>&1 && fail "overlay ran with PRIVATE_PATHS unset"
mv "$TMP/conf.bak" .claude/private-overlay.conf
sh scripts/private-overlay.sh bogus >/dev/null 2>&1 && fail "overlay accepted an unknown subcommand"
pass "overlay rejects a missing conf, unset paths and unknown subcommands"

# spec: 006/FR-011
sh scripts/private-overlay.sh init >/dev/null
[ -z "$(git --git-dir=.private.git remote)" ] || fail "init added a remote from the {{placeholder}} conf"
rm -rf .private.git && git checkout -q -- .gitignore
pass "placeholder conf means no remote"

REMOTE="$TMP/private-remote.git"
git init -q --bare "$REMOTE"
sed "s#^PRIVATE_REMOTE=.*#PRIVATE_REMOTE=\"$REMOTE\"#" .claude/private-overlay.conf > conf.tmp
mv conf.tmp .claude/private-overlay.conf
echo '{}' > .claude/settings.local.json   # machine-local: must never be mirrored

# spec: 006/FR-002, 006/FR-003
sh scripts/private-overlay.sh init >/dev/null
grep -q '^# >>> private-overlay >>>$' .gitignore || fail "init: no managed .gitignore block"
grep -q '^CLAUDE.md$' .gitignore || fail "init: CLAUDE.md not in .gitignore block"
sh scripts/private-overlay.sh init >/dev/null && sh scripts/private-overlay.sh gitignore >/dev/null
[ "$(grep -c '^# >>> private-overlay >>>$' .gitignore)" -eq 1 ] || fail "re-init/gitignore appended a second block"
pass "init patches .gitignore"

# spec: 006/FR-004, 006/FR-007
sh scripts/private-overlay.sh commit "smoke" >/dev/null
sh scripts/private-overlay.sh push >/dev/null 2>&1
tracked="$(git --git-dir="$REMOTE" ls-tree -r --name-only main)"
echo "$tracked" | grep -qx CLAUDE.md || fail "overlay remote missing CLAUDE.md"
echo "$tracked" | grep -qx .claude/settings.local.json && fail "overlay mirrored settings.local.json"
echo "$tracked" | grep -q '^scripts/' && fail "overlay tracked a non-private path"
pass "commit + push mirror only private paths"

# spec: 006/FR-006, 006/FR-007, 006/FR-009
sh scripts/private-overlay.sh | grep -q . || fail "no subcommand didn't run status"
echo more >> CLAUDE.md && sh scripts/private-overlay.sh commit >/dev/null
git --git-dir=.private.git log -1 --format=%s | grep -Eq '^overlay [0-9]{4}-[0-9]{2}-[0-9]{2}T' || fail "commit without a message isn't 'overlay <UTC timestamp>'"
sh scripts/private-overlay.sh push >/dev/null 2>&1
git clone -q -b main "$REMOTE" "$TMP/other" && echo "from elsewhere" >> "$TMP/other/CLAUDE.md"
git -C "$TMP/other" commit -qam elsewhere && git -C "$TMP/other" push -q origin HEAD
sh scripts/private-overlay.sh pull >/dev/null 2>&1 || fail "overlay pull failed"
grep -q "from elsewhere" CLAUDE.md || fail "overlay pull didn't bring the remote change"
pass "default commit message; pull fast-forwards"

# spec: 006/FR-008
rm CLAUDE.md
sh scripts/private-overlay.sh restore | grep -q "restored: CLAUDE.md" || fail "restore didn't report CLAUDE.md"
[ -f CLAUDE.md ] || fail "restore didn't bring back CLAUDE.md"
sh scripts/private-overlay.sh restore | grep -q "nothing missing" || fail "restore not a no-op"
pass "restore recovers a deleted file, then no-ops"

# spec: 006/FR-005
# an unreadable file makes only `git add` fail (not `git commit`) — the
# overlay must stop rather than commit the other paths' changes without it
echo change >> CLAUDE.md
echo change >> CONTEXT.md
chmod 000 CONTEXT.md
before="$(git --git-dir=.private.git rev-parse HEAD)"
sh scripts/private-overlay.sh commit "should fail" >/dev/null 2>&1 || true
chmod 644 CONTEXT.md
[ "$(git --git-dir=.private.git rev-parse HEAD)" = "$before" ] || fail "overlay committed despite a failed git add"
pass "a failed git add stops the overlay commit"
fi

# --- check-tooling.sh -------------------------------------------------------
# optional section comes from the catalog's `check` lines; exit status ignored
# (required tools aren't installed in CI)
# spec: 001/FR-009
if [ -f .claude/tooling-catalog.yaml ]; then
printf 'yes-tool:\n  purpose: always on\n  check: true\nno-tool:\n  purpose: never on\n  check: false\n' > .claude/tooling-catalog.yaml
out="$(sh scripts/check-tooling.sh 2>&1 || true)"
echo "$out" | grep -q "yes-tool" || fail "check-tooling skipped an active catalog tool"
echo "$out" | grep -q "no-tool" && fail "check-tooling listed an inactive catalog tool"
pass "check-tooling derives optional tools from the catalog"

cp "$SRC"/.claude/tooling-catalog.yaml .claude/
fi

# --- workflow toolchains (spec 017) -----------------------------------------
cat=.claude/tooling-catalog.yaml reg=.claude/skills-registry.yaml
tab="$(printf '\t')"
# catalog → one record per entry, empty field = "-": name kind required provides relevant_when check install purpose
records() {
  awk 'function out() { if (n != "") { printf "%s", n; for (i = 1; i <= 7; i++) printf "\t%s", (K[i] in f && f[K[i]] != "" ? f[K[i]] : "-"); print "" } }
       BEGIN { split("kind required provides relevant_when check install purpose", K, " ") }
       /^#/ { next }
       /^[a-z0-9-]+:$/ { out(); n = substr($0, 1, length($0) - 1); split("", f); next }
       /^  [a-z_]+: / { k = $1; sub(/:$/, "", k); v = $0; sub(/^  [a-z_]+: /, "", v); f[k] = v }
       END { out() }' "$1"
}
chain() { awk -v k="$1" '/^[^ #]/ { on = ($0 ~ "^" k ":") } on && /^    - / { sub(/^    - /, ""); print }' "$2"; }
if [ -f "$cat" ]; then
recs="$(records "$cat")"
# spec: 017/FR-002
bad="$(printf '%s\n' "$recs" | awk -F'\t' '$2 == "-" || $6 == "-" || ($7 == "-" && $2 != "builtin") || $8 == "-" || ($3 != "-" && $3 != "global") { print $1 }')"
[ -z "$bad" ] || fail "catalog entries missing kind/check/install/purpose or bad required: $bad"
# spec: 001/FR-001
noadd="$(awk '/^[a-z0-9-]+:$/ { if (n != "" && !a) print n; n = $0; a = 0 } /^  added_in: [0-9]+\.[0-9]+\.[0-9]+$/ { a = 1 } END { if (n != "" && !a) print n }' "$cat")"
[ -z "$noadd" ] || fail "catalog entries without added_in: $noadd"
# spec: 001/FR-011
if [ -n "$OAIKIT" ]; then
  rel="$(printf '%s\n' "$recs" | awk -F'\t' '$1 == "rea" { print $5 }')"
  case "$rel" in *"copy, clone, or lookalike of an existing app or site"*"without its source"*) ;; *) fail "rea missing or its triggers changed" ;; esac
fi
pass "catalog entries are complete"

# spec: 001/FR-010
if [ -n "$OAIKIT" ]; then   # derived projects' READMEs are their own (project wizard)
  while IFS="$tab" read -r name _ req _ rel _; do
    if [ "$req" = global ]; then row="| $name |"; elif [ "$rel" != - ]; then row="| $name *(optional)* |"; else continue; fi
    grep -qF "$row" "$SRC/README.md" || fail "README tooling table missing $row"
  done <<EOF
$recs
EOF
  pass "README lists every global and optional catalog tool"
fi

if [ -f "$reg" ]; then
provides=" $(printf '%s\n' "$recs" | cut -f4 | tr ',' '\n' | tr -d ' ' | grep -v '^-$' | tr '\n' ' ') "
names=" $(printf '%s\n' "$recs" | cut -f1 | tr '\n' ' ') "
for k in $(sed -n 's/^\([a-z0-9-]*\):.*/\1/p' "$reg"); do
  steps="$(chain "$k" "$reg")"
  # spec: 017/FR-001
  [ -n "$steps" ] || fail "registry $k has no chain:"
  err="$(printf '%s\n' "$steps" | awk -F' [|] ' -v k="$k" '
    BEGIN { n = split("explore spec plan build verify review ship", P, " "); for (i = 1; i <= n; i++) r[P[i]] = i }
    NF != 5 { print k ": step needs 5 fields: " $0; next }
    !($1 in r) { print k ": unknown phase: " $0; next }
    r[$1] < last { print k ": phase out of order: " $0 }
    { last = r[$1] }
    $4 != "required" && $4 != "optional" { print k ": required|optional: " $0 }
    $4 == "required" && $5 == "-" { print k ": required step without fallback: " $0 }
    $4 == "optional" && $5 != "-" { print k ": optional step fallback must be -: " $0 }')"
  [ -z "$err" ] || fail "$err"
  # spec: 017/FR-002
  for t in $(printf '%s\n' "$steps" | awk -F' [|] ' '{ print $2 }'); do
    case "$names$provides project-checks " in *" $t "*) ;; *) fail "registry $k names unknown tool: $t" ;; esac
  done
  # spec: 017/FR-003
  w="$(awk -v k="$k" '/^[^ #]/ { on = ($0 ~ "^" k ":") } on && /^  workflow: / { print $2; exit }' "$reg")"
  exp="$(printf '<!-- toolchain -->\n| Phase | Tool | When | Fallback |\n|---|---|---|---|\n'
         printf '%s\n' "$steps" | awk -F' [|] ' -v sk="$provides" '{ t = $2; if (index(sk, " " t " ")) t = "`/" t "`"; if ($4 == "required") t = t " (required)"; print "| " $1 " | " t " | " $3 " | " $5 " |" }'
         echo '<!-- /toolchain -->')"
  got="$(sed -n '/^<!-- toolchain -->$/,/^<!-- \/toolchain -->$/p' "$w")"
  [ "$exp" = "$got" ] || { printf '%s: Toolchain block differs from the registry; expected:\n%s\n' "$w" "$exp" >&2; fail "$k toolchain doc out of sync"; }
done
pass "every workflow has a valid chain, tools resolve, docs match"

# spec: 017/FR-009, 017/FR-010
if [ -n "$OAIKIT" ]; then
  has() { chain "$1" "$reg" | awk -F' [|] ' '{ print $2 }' | grep -qx "$2" || fail "$1 chain lacks $2"; }
  for t in diagnosing-bugs tokensave grill-with-docs speckit-specify speckit-clarify speckit-plan speckit-tasks speckit-analyze tdd frontend-design rea spec-audit code-review ponytail-review security-review speckit-taskstoissues; do has coding "$t"; done
  # spec: 018/FR-001, 018/FR-005
  for t in research book-genesis beta-reader literary-agent-panel editorial-package; do has book "$t"; done
  chain book "$reg" | grep -q '^spec | book-genesis | .* | required | stop and ask$' || fail "book's spec step must require book-genesis, fallback stop and ask"
  has content beta-reader
  # spec: 018/FR-002
  printf '%s\n' "$recs" | awk -F'\t' '$1 == "book-genesis" && $4 ~ /beta-reader/ && $5 ~ /novel/ && $6 ~ /skills\/book-genesis/' | grep -q . || fail "catalog book-genesis entry incomplete"
  has docs-driven graphify; has docs-driven grill-me; has infra project-checks; has infra security-review; has agentic claude-api; has agentic tdd
  pass "chains carry the agreed tools"
fi

# spec: 017/FR-008, 017/FR-012, 011/FR-001, 011/FR-002
cp "$cat" "$TMP/cat.bak" && cp "$reg" "$TMP/reg.bak"
cat > "$cat" <<'EOF'
g-ok:
  kind: cli
  required: global
  purpose: global on
  install: x
  check: true
plug:
  kind: plugin
  provides: tdd, grill-me
  purpose: skill pack
  install: /plugin install plug
  check: false
base:
  kind: builtin
  provides: code-review
  purpose: built in
  install: -
  check: true
comp:
  kind: component
  provides: speckit-plan
  purpose: unused component
  install: x
  check: false
opt-off:
  kind: plugin
  purpose: optional off
  relevant_when: never
  install: x
  check: false
EOF
cat > "$reg" <<'EOF'
coding:
  workflow: docs/workflows/coding.md
  chain:
    - plan | speckit-plan | spec-kit project | required | stop and ask
    - build | tdd | always | required | write the test by hand
    - review | code-review | always | required | review it yourself
    - review | grill-me | sometimes | optional | -
infra:
  workflow: docs/workflows/infra.md
  chain:
    - verify | project-checks | always | required | run them by hand
EOF
printf 'skills:\n  - coding\n  - infra\n  - ghost\nrequired_checks:\n  - lint\n' > "$TMP/ct.yaml"
out="$(PROFILE="$TMP/ct.yaml" sh scripts/check-tooling.sh 2>&1)" && fail "check-tooling passed with a required plugin missing"
echo "$out" | grep -q 'Required by coding' || fail "check-tooling has no per-workflow section"
echo "$out" | grep '✗' | grep -q 'tdd (plug)' || fail "check-tooling didn't report the missing plugin under coding"
echo "$out" | grep '✔' | grep -q 'code-review (base)' || fail "check-tooling reported a builtin as missing"
echo "$out" | grep 'speckit-plan (comp)' | grep -q 'n/a' || fail "check-tooling didn't mark an unused component n/a"
echo "$out" | grep -q 'grill-me' && fail "check-tooling listed an optional step as required"
echo "$out" | grep 'project-checks' | grep -q lint || fail "check-tooling didn't show the profile's required_checks"
echo "$out" | grep -q 'ghost' || fail "check-tooling didn't note an unregistered skill"
echo "$out" | grep -q 'opt-off' && fail "check-tooling listed an inactive optional tool"
sed -i.bak 's/^  check: false$/  check: true/' "$cat"
PROFILE="$TMP/ct.yaml" sh scripts/check-tooling.sh >/dev/null 2>&1 || fail "check-tooling failed with every required tool active"
cp "$TMP/cat.bak" "$cat" && cp "$TMP/reg.bak" "$reg" && rm -f "$cat.bak"
pass "check-tooling reports required tools per enabled workflow"
fi
fi

# spec: 012/FR-002
if [ -n "$OAIKIT" ]; then   # every script OAIKit ships is a template script, so it gets linted
  for f in scripts/*.sh; do
    case " $TEMPLATE_SCRIPTS " in *" ${f#scripts/} "*) ;; *) fail "TEMPLATE_SCRIPTS is missing ${f#scripts/}" ;; esac
  done
  pass "TEMPLATE_SCRIPTS covers every shipped script"
fi

# --- profile-get.sh ---------------------------------------------------------
# spec: 016/FR-004
P="$TMP/profile.yaml"
cat > "$P" <<'EOF'
skills:
  - infra
  - coding   # comment
branching: trunk   # comment
default_branch: master
components:
  merge-guard: false
  private-overlay: true
tooling:
  rea: declined
template_version: 0.10.0
EOF
pg() { PROFILE="$P" sh scripts/profile-get.sh "$@"; }
[ "$(pg branching)" = trunk ] || fail "profile-get scalar"
[ "$(pg skills | tr '\n' ' ')" = "infra coding " ] || fail "profile-get list items, one per line"
[ "$(pg default_branch)" = master ] || fail "profile-get scalar with following map"
[ "$(pg components.merge-guard)" = false ] || fail "profile-get map value"
[ "$(pg tooling.rea)" = declined ] || fail "profile-get second map"
[ "$(pg components.docs-site true)" = true ] || fail "profile-get absent map entry -> default"
[ "$(pg autonomy supervised)" = supervised ] || fail "profile-get absent key -> default"
[ "$(PROFILE=/nonexistent sh scripts/profile-get.sh branching gitflow)" = gitflow ] || fail "profile-get missing file -> default"
PROFILE=/nonexistent sh scripts/profile-get.sh branching >/dev/null || fail "profile-get must exit 0"
pass "profile-get reads scalars, maps and defaults"

# --- scripts.yml ------------------------------------------------------------
# spec: 012/FR-001
[ -f .github/workflows/scripts.yml ] && grep -q 'paths:' .github/workflows/scripts.yml && fail "scripts.yml has a paths filter (smoke inputs live outside scripts/)"
# spec: 016/FR-009
# ponytail: template workflows only — a project's own CI (ci.yml etc.) may filter on its branches
for w in .github/workflows/main-merge-guard.yml .github/workflows/scripts.yml .github/workflows/docs-site.yml; do
  [ -f "$w" ] || continue
  grep -qE 'branches: \[(main|master)' "$w" && fail "$w hardcodes a default-branch name"
done
# spec: 013/FR-001 — hashFiles() in a job-level if makes GitHub reject the whole workflow
for w in .github/workflows/*.yml; do
  grep -Eq '^    if: .*hashFiles\(' "$w" && fail "$w uses hashFiles() in a job-level if (invalid workflow)"
done
pass "script CI runs on every PR"

# --- main-merge-guard.yml ---------------------------------------------------
# spec: 016/FR-011
g=.github/workflows/main-merge-guard.yml
if [ -f "$g" ]; then
grep -q 'github.event.repository.default_branch' "$g" || fail "merge guard doesn't read the repo's default branch"
grep -q 'branches: \[main\]' "$g" && fail "merge guard hardcodes main"
pass "merge guard follows the default branch"
fi

# --- repo-setup.sh ----------------------------------------------------------
# spec: 016/FR-010, 007/FR-005
printf 'branching: trunk\ndefault_branch: master\n' > "$TMP/trunk.yaml"
RS="$TMP/rs" && git clone -q . "$RS" && cp -p scripts/*.sh "$RS/scripts/" && git -C "$RS" remote remove origin
out="$(cd "$RS" && PROFILE="$TMP/trunk.yaml" sh scripts/repo-setup.sh --check)" || fail "repo-setup without origin must exit 0"
echo "$out" | grep -q 'branching=trunk default=master' || fail "repo-setup ignores the profile's branching/default branch"
echo "$out" | grep -q 'skipped' || fail "repo-setup without origin didn't skip"
# spec: 016/FR-009
printf 'branching: gitflow\ndefault_branch: master\n' > "$TMP/gf.yaml"
(cd "$RS" && PROFILE="$TMP/gf.yaml" sh scripts/repo-setup.sh --check) | grep -q 'branching=gitflow default=master' || fail "repo-setup ignores a gitflow profile's default branch"
git -C "$RS" remote add origin "file://$TMP/nowhere.git"
(cd "$RS" && sh scripts/repo-setup.sh --check) | grep -q 'not GitHub' || fail "repo-setup didn't skip a non-GitHub origin"
(cd "$RS" && sh scripts/repo-setup.sh --bogus >/dev/null 2>&1) && fail "repo-setup accepted an unknown argument"
# spec: 007/FR-003, 007/FR-005
GHB="$TMP/ghbin" && mkdir -p "$GHB" && export GHLOG="$TMP/gh.log"
cat > "$GHB/gh" <<'EOF'
#!/bin/sh
echo "$*" >> "$GHLOG"
case "$*" in
  "repo view --json nameWithOwner"*) echo t/x ;;
  *deleteBranchOnMerge*) [ -n "${GH_OK:-}" ] && echo true || echo false ;;
  *"/protection --jq"*) [ -n "${GH_OK:-}" ] && echo "main-merge-guard true" || { echo "gh: Branch not protected (HTTP 404)"; exit 1; } ;;
  "label list"*) echo bug; [ -n "${GH_OK:-}" ] && echo chore ;;
esac
EOF
chmod +x "$GHB/gh"
git init -q --bare "$TMP/github.com/x.git"
git -C "$RS" remote set-url origin "file://$TMP/github.com/x.git" && git -C "$RS" push -q origin HEAD:refs/heads/master
out="$(cd "$RS" && PROFILE="$TMP/gf.yaml" PATH="$GHB:$PATH" sh scripts/repo-setup.sh --check)"
for w in "develop *MISSING" "delete-on-merge *OFF" "merge-guard *OFF" "chore-label *MISSING" "changes pending"; do
  echo "$out" | grep -q "$w" || { echo "$out" >&2; fail "repo-setup --check did not report: $w"; }
done
grep -Eq 'edit|-X PUT|label create' "$GHLOG" && fail "repo-setup --check changed something: $(cat "$GHLOG")"
git ls-remote --exit-code --heads "$TMP/github.com/x.git" develop >/dev/null && fail "repo-setup --check created develop"
git -C "$RS" push -q origin HEAD:refs/heads/develop
out="$(cd "$RS" && GH_OK=1 PROFILE="$TMP/gf.yaml" PATH="$GHB:$PATH" sh scripts/repo-setup.sh --check)"
echo "$out" | grep -q "changes pending" && fail "repo-setup --check reported pending on a set-up repo"
NOGH="$TMP/nogh" && mkdir -p "$NOGH"
for t in git sed awk grep paste cat dirname tr basename; do ln -s "$(command -v $t)" "$NOGH/$t"; done
(cd "$RS" && PATH="$NOGH" /bin/sh scripts/repo-setup.sh --check >/dev/null 2>&1) && fail "repo-setup without gh must exit 1"
pass "repo-setup reads the profile and skips non-GitHub repos"

# --- skills registry / profile -----------------------------------------------
# spec: 003/FR-001, 003/FR-002, 003/FR-003, 016/FR-005, 016/FR-023
reg=.claude/skills-registry.yaml
for w in $(sed -n 's/^  workflow: //p' "$reg"); do
  [ -f "$w" ] || fail "registry workflow missing: $w"
  head -5 "$w" | grep -q '^\*\*Kicks in when:\*\*' || fail "$w has no 'Kicks in when' line near the top"
done
# unknown names (e.g. removed from the template) are reported, not fatal — 016/FR-023
for k in $(sh scripts/profile-get.sh skills); do
  grep -q "^$k:" "$reg" || echo "note: profile skill '$k' isn't in the registry — ignored"
done
printf 'skills:\n  - coding\n  - retired-skill\n' > "$TMP/unk.yaml"
PROFILE="$TMP/unk.yaml" sh scripts/profile-get.sh skills | grep -qx retired-skill || fail "profile-get dropped an unknown skill"
if [ -n "$OAIKIT" ]; then
  for k in infra agentic book; do grep -q "^$k:" "$reg" || fail "registry missing $k"; done
fi
pass "registry entries resolve; profile skills are registered"

# --- settings, releases, sync lists -------------------------------------------
st=.claude/settings.json
if [ -f "$st" ] && command -v python3 >/dev/null 2>&1; then
# spec: 007/FR-008, 010/FR-005
python3 - "$st" <<'EOF' || fail "settings.json: bad worktree setting or caveman hook"
import json, subprocess, sys
s = json.load(open(sys.argv[1]))
assert s["worktree"]["bgIsolation"] == "none", "bgIsolation"
h = s["hooks"]["UserPromptSubmit"][0]["hooks"][0]
assert h["type"] == "command" and h["timeout"] == 5, "hook shape"
out = json.loads(subprocess.run(["sh", "-c", h["command"]], capture_output=True, text=True, check=True).stdout)
o = out["hookSpecificOutput"]
assert o["hookEventName"] == "UserPromptSubmit" and "CAVEMAN MODE" in o["additionalContext"], "hook output"
EOF
pass "settings: in-place sessions, caveman hook emits valid JSON"
fi

if [ -n "$OAIKIT" ]; then
# spec: 004/FR-011, 008/FR-006
for f in docs/releases/v*.md; do
  v="$(basename "$f" .md)"
  grep -q '^## Highlights' "$f" && grep -q '^## Upgrading' "$f" || fail "$f lacks Highlights/Upgrading"
  grep -q "^| \[${v#v}\](docs/releases/$v.md)" CHANGELOG.md || fail "CHANGELOG has no row for $v"
done
grep '^| ' CHANGELOG.md | sed -n 2p | grep -q '^| Unreleased | — |' || fail "CHANGELOG's first row isn't Unreleased"
# spec: 008/FR-008
[ "$(cat VERSION)" = "$(sh scripts/profile-get.sh template_version)" ] || fail "VERSION != profile template_version"
pass "release docs, CHANGELOG rows and VERSION agree"

# spec: 014/FR-005 (skill types; tooling rows and private paths are checked above)
for k in $(sed -n 's/^\([a-z0-9-]*\):.*/\1/p' "$reg"); do
  grep -q "\`$k\`" "$SRC/CLAUDE.md" || fail "CLAUDE.md Dev Workflow doesn't name skill type $k"
  grep -q "\`$k\`" "$SRC/README.md" || fail "README Skill profiles doesn't name skill type $k"
done
# spec: 015/FR-004
sec="$(sed -n '/^## What.s in here/,/^## [^W]/p' "$SRC/README.md")"
for p in $(git -C "$SRC" ls-files | awk -F/ 'NF == 1 { print; next } $1 == ".claude" && $2 == "skills" { print $1 "/" $2 "/" $3; next } $1 == ".github" { print $1 "/" $2 "/" $3; next } { print $1 "/" $2 }' | sort -u); do
  case "$p" in .gitattributes|.gitignore|LICENSE|README.md|.claude/skills/speckit-*|specs/*|.specify/*) continue ;; esac
  echo "$sec" | grep -qF "$p" || fail "README \"What's in here\" doesn't list $p"
done
pass "sync lists: skill types named, README lists every shipped path"
fi

# --- derived-project shape (#51) ---------------------------------------------
# spec: 016/FR-014
# Re-run this whole smoke test inside a project shaped like a real derived one: gitignored
# CLAUDE.md, no overlay conf, no OAIKit-only files, and its own (broken) script. Must pass.
# ponytail: OAIKit only — a derived project already is that shape (and its .claude/ isn't tracked)
if [ -z "${SMOKE_NESTED:-}" ] && [ -n "$OAIKIT" ]; then
  DP="$TMP/derived" && git clone -q . "$DP"
  cp -p scripts/*.sh "$DP/scripts/" && cp .claude/tooling-catalog.yaml .claude/skills-registry.yaml "$DP/.claude/" && cp .github/workflows/*.yml "$DP/.github/workflows/"
  cp docs/workflows/*.md "$DP/docs/workflows/"
  (
    cd "$DP"
    printf 'CLAUDE.md\n' >> .gitignore && git rm -q --cached CLAUDE.md
    git rm -rq --ignore-unmatch docs/maintaining.md specs .specify .claude/private-overlay.conf >/dev/null
    printf 'if then\n' > scripts/dev-up.sh   # project-owned, not the template's: never linted
    git add -A && git commit -qm "derived shape"
  )
  (cd "$DP" && SMOKE_NESTED=1 sh scripts/smoke-test.sh >"$TMP/nested.log" 2>&1) || { cat "$TMP/nested.log" >&2; fail "smoke test fails in a derived-shaped project"; }
  pass "smoke test passes in a derived-shaped project"
fi

echo "all smoke tests passed"
