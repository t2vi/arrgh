#!/bin/sh
# Snapshots this repo's gitignored-but-real docs into a local-only git repo —
# separate git dir, same working tree, no remote, ever. Run before any
# operation that could touch these paths (a merge, MIGRATE.md, a rebase):
# they have no object in the main repo's history for git to fall back on if
# something overwrites them — see MIGRATE.md's own incident note for why
# that's not hypothetical.
#
# Default path list is what OAIKit itself manages as "living docs" — extend
# it below for anything else this project keeps out of git on purpose.
set -eu

WORK_TREE="$(git rev-parse --show-toplevel)"
REPO_NAME="$(basename "$WORK_TREE")"
BACKUP_GIT_DIR="${1:-$HOME/Backups/${REPO_NAME}-private.git}"

PATHS="
CLAUDE.md
CONTEXT.md
docs/adr
.claude/profile.yaml
.claude/settings.json
.claude/settings.local.json
.claude/skills-registry.yaml
"

mkdir -p "$(dirname "$BACKUP_GIT_DIR")"
[ -d "$BACKUP_GIT_DIR" ] || git init -q --bare "$BACKUP_GIT_DIR"

b() { git --git-dir="$BACKUP_GIT_DIR" --work-tree="$WORK_TREE" "$@"; }

cd "$WORK_TREE"
for p in $PATHS; do
  [ -e "$p" ] && b add -f "$p"
done

if [ -n "$(b diff --cached --name-only 2>/dev/null)" ]; then
  b commit -q -m "backup $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "Backed up to $BACKUP_GIT_DIR"
else
  echo "No changes since last backup ($BACKUP_GIT_DIR)"
fi
