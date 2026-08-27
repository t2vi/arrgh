#!/bin/sh
# Private overlay — a second git repo layered over this working tree that
# tracks ONLY the AI/dev files listed in .claude/private-overlay.conf and
# syncs them to a private remote. The files live in the real repo on disk
# (so no symlinks, no sibling clone, tools see them where they expect) but
# never enter the public repo's history — the public .gitignore excludes
# them, and this overlay's git-dir (.private.git/) is itself ignored.
#
# Replaces the older "symlink everything into a sibling private repo" setup.
#
# NOTE: if the public repo already TRACKS a path that moves into the overlay,
# adding it to .gitignore is not enough — `git rm --cached <path>` it from the
# public repo too. Paths that were only symlinked (untracked) need nothing.
#
#   scripts/private-overlay.sh init            # create .private.git, wire remote, patch .gitignore
#   scripts/private-overlay.sh status          # what changed in the tracked AI files
#   scripts/private-overlay.sh commit [msg]    # stage the tracked paths + commit
#   scripts/private-overlay.sh push            # push to the private remote (normal, non-forced)
#   scripts/private-overlay.sh pull            # ff-only pull from the private remote
#
# First-time population of an existing private remote you want to OVERWRITE
# (e.g. migrating off the old symlink repo) is a deliberate manual step:
#   scripts/private-overlay.sh init
#   scripts/private-overlay.sh commit "init: private overlay"
#   git --git-dir=.private.git push -f origin main
#
# ponytail: plain git with a separate --git-dir + core.worktree. No deps,
# no daemon. core.worktree is relative ("..") so moving the repo is fine.
set -eu

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
GD="$ROOT/.private.git"
CONF="$ROOT/.claude/private-overlay.conf"

[ -f "$CONF" ] || { echo "private-overlay: missing $CONF" >&2; exit 1; }
# shellcheck disable=SC1090
. "$CONF"
: "${PRIVATE_PATHS:?private-overlay: PRIVATE_PATHS unset in $CONF}"

p() { git --git-dir="$GD" --work-tree="$ROOT" "$@"; }

# `git add -f` is required so the overlay can track paths the PUBLIC repo's
# .gitignore deliberately excludes (that's the whole point). -f also defeats
# info/exclude, so junk (dep/build trees, the host .git, editor cruft) is
# filtered back out with :(exclude) pathspecs instead.
OVERLAY_EXCLUDES=":(exclude,glob)**/node_modules/**
:(exclude,glob)**/.git/**
:(exclude,glob)**/dist/**
:(exclude,glob)**/build/**
:(exclude,glob)**/.venv/**
:(exclude,glob)**/__pycache__/**
:(exclude,glob)**/*.pyc
:(exclude,glob)**/.DS_Store"

stage() {
  # shellcheck disable=SC2086
  for x in $PRIVATE_PATHS; do
    [ -e "$ROOT/$x" ] && p add -f "$x" $OVERLAY_EXCLUDES || true
  done
  # never mirror machine-local Claude state, even when a parent dir is listed
  for x in ${PRIVATE_EXCLUDE:-.claude/settings.local.json .claude/projects .claude/worktrees .claude/scheduled_tasks.lock .claude/.credentials.json}; do
    p rm -r --cached --ignore-unmatch --quiet "$x" 2>/dev/null || true
  done
}

patch_gitignore() {
  gi="$ROOT/.gitignore"
  tmp="$gi.private-overlay.tmp"
  touch "$gi"
  # drop any previous managed block, then re-append a fresh one
  awk '
    /^# >>> private-overlay >>>$/ {skip=1}
    !skip {print}
    /^# <<< private-overlay <<<$/ {skip=0}
  ' "$gi" > "$tmp"
  {
    printf '\n# >>> private-overlay >>>\n'
    printf '# AI/dev files tracked in the private overlay (scripts/private-overlay.sh) — never public.\n'
    printf '# Regenerated from .claude/private-overlay.conf; edit that, not this block.\n'
    printf '.private.git/\n.private-tmp/\n'
    for x in $PRIVATE_PATHS; do printf '%s\n' "$x"; done
    printf '# <<< private-overlay <<<\n'
  } >> "$tmp"
  mv "$tmp" "$gi"
}

case "${1:-status}" in
  init)
    if [ ! -d "$GD" ]; then
      git --git-dir="$GD" init -q
      p config core.bare false
      p config core.worktree ..
      printf '*\n' > "$GD/info/exclude"
      p symbolic-ref HEAD refs/heads/main
      [ -n "${PRIVATE_REMOTE:-}" ] && p remote add origin "$PRIVATE_REMOTE" || true
      # init never fetches/checks out — run `private-overlay.sh pull` to adopt
      # existing remote history, or just `commit` + force-push to replace it.
    fi
    patch_gitignore
    echo "private-overlay: ready ($GD). Tracked paths patched into .gitignore."
    ;;
  status) stage; p -c status.showUntrackedFiles=no status -sb ;;
  commit) shift; stage; p commit -m "${*:-overlay $(date -u +%FT%TZ)}" ;;
  push)   p push -u origin HEAD ;;
  pull)   p pull --ff-only origin "$(p rev-parse --abbrev-ref HEAD 2>/dev/null || echo main)" ;;
  gitignore) patch_gitignore; echo "private-overlay: .gitignore block regenerated." ;;
  *) echo "usage: private-overlay.sh {init|status|commit [msg]|push|pull|gitignore}" >&2; exit 1 ;;
esac
