#!/bin/sh
# Snapshots this repo's gitignored-but-real docs into a local-only git repo —
# separate git dir, same working tree, no remote, ever. Run before any
# operation that could touch these paths (a merge, MIGRATE.md, a rebase):
# they have no object in the main repo's history for git to fall back on if
# something overwrites them — see MIGRATE.md's own incident note for why
# that's not hypothetical.
#
# Default path list is what OAIKit itself manages as "living docs" — a
# superset of PRIVATE_PATHS in .claude/private-overlay.conf (smoke test
# enforces), minus .claude's machine-local junk (EXCLUDE). Extend it below
# for anything else this project keeps out of git on purpose.
#
# Restore (default location shown; pass the same path you backed up to):
#   git --git-dir="$HOME/Backups/<repo>-private.git" log --oneline -- <path>        # find the snapshot
#   git --git-dir="$HOME/Backups/<repo>-private.git" --work-tree=. checkout <sha> -- <path>
set -eu

WORK_TREE="$(git rev-parse --show-toplevel)"
REPO_NAME="$(basename "$WORK_TREE")"
BACKUP_GIT_DIR="${1:-$HOME/Backups/${REPO_NAME}-private.git}"

PATHS="
CLAUDE.md
CONTEXT.md
docs/adr
docs/agents
.claude
"
# machine-local or bulky — never backed up (settings.local.json is kept on purpose: local-only repo)
EXCLUDE=".claude/projects .claude/worktrees .claude/scheduled_tasks.lock .claude/.credentials.json"

mkdir -p "$(dirname "$BACKUP_GIT_DIR")"
[ -d "$BACKUP_GIT_DIR" ] || git init -q --bare "$BACKUP_GIT_DIR"

b() { git --git-dir="$BACKUP_GIT_DIR" --work-tree="$WORK_TREE" "$@"; }

cd "$WORK_TREE"
for p in $PATHS; do
  # shellcheck disable=SC2046 # one :(exclude) pathspec per EXCLUDE word
  [ -e "$p" ] && b add -f "$p" $(for x in $EXCLUDE; do printf ':(exclude)%s ' "$x"; done)
done

if [ -n "$(b diff --cached --name-only 2>/dev/null)" ]; then
  b commit -q -m "backup $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "Backed up to $BACKUP_GIT_DIR"
else
  echo "No changes since last backup ($BACKUP_GIT_DIR)"
fi
