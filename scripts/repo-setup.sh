#!/bin/sh
# Brings this project's GitHub repo onto the gitflow setup in docs/gitflow.md.
# Idempotent — safe to re-run; each step is skipped when already in place:
#
#   1. `develop` branch exists (created from origin/main if missing)
#   2. delete-branch-on-merge is on
#   3. `main` is protected by the merge guard (required main-merge-guard
#      check, PRs only, admins included) — only once
#      .github/workflows/main-merge-guard.yml is on origin/main; before that a
#      hotfix branched from main would wait forever on a check that never runs.
#
#   scripts/repo-setup.sh --check   # report only, changes nothing
#   scripts/repo-setup.sh           # apply what's missing (changes GitHub — ask first)
#
# GitHub only: any other origin (Azure DevOps, GitLab, none) is skipped.
set -eu

MODE="${1:-apply}"
case "$MODE" in apply|--check) ;; *) echo "usage: repo-setup.sh [--check]" >&2; exit 1 ;; esac

url="$(git remote get-url origin 2>/dev/null)" || { echo "repo-setup: no origin remote — skipped."; exit 0; }
case "$url" in
  *github.com*) ;;
  *) echo "repo-setup: origin is not GitHub ($url) — skipped (GitHub-only)."; exit 0 ;;
esac
command -v gh >/dev/null 2>&1 || { echo "repo-setup: gh CLI not found" >&2; exit 1; }

repo="$(gh repo view --json nameWithOwner -q .nameWithOwner)"
git fetch -q origin
pending=0
say() { printf '  %-22s %s\n' "$1" "$2"; }
echo "repo-setup: $repo ($MODE)"

# 1. develop
if git ls-remote --exit-code --heads origin develop >/dev/null 2>&1; then
  say develop "ok"
elif [ "$MODE" = --check ]; then
  say develop "MISSING — would create from origin/main"; pending=1
else
  git push -q origin "refs/remotes/origin/main:refs/heads/develop"
  say develop "created from origin/main"
fi

# 2. delete-branch-on-merge
if [ "$(gh repo view "$repo" --json deleteBranchOnMerge -q .deleteBranchOnMerge)" = true ]; then
  say delete-on-merge "ok"
elif [ "$MODE" = --check ]; then
  say delete-on-merge "OFF — would enable"; pending=1
else
  gh repo edit "$repo" --delete-branch-on-merge >/dev/null
  say delete-on-merge "enabled"
fi

# 3. main merge guard
prot="$(gh api "repos/$repo/branches/main/protection" --jq '.required_status_checks.contexts // [] | join(",")' 2>&1)" || true
case "$prot" in
  *main-merge-guard*)
    say merge-guard "ok" ;;
  *"Upgrade to GitHub Pro"*|*"HTTP 403"*)
    say merge-guard "unavailable — plan doesn't support branch protection on this repo" ;;
  *)
    if ! git cat-file -e "origin/main:.github/workflows/main-merge-guard.yml" 2>/dev/null; then
      say merge-guard "PENDING — workflow not on main yet; re-run after the next release"; pending=1
    elif [ "$MODE" = --check ]; then
      say merge-guard "OFF — would protect main"; pending=1
    else
      gh api -X PUT "repos/$repo/branches/main/protection" --input - >/dev/null <<'JSON'
{
  "required_status_checks": { "strict": false, "contexts": ["main-merge-guard"] },
  "enforce_admins": true,
  "required_pull_request_reviews": { "required_approving_review_count": 0 },
  "restrictions": null
}
JSON
      say merge-guard "main protected"
    fi ;;
esac

if [ "$MODE" = --check ] && [ "$pending" -eq 1 ]; then
  echo "repo-setup: changes pending — run scripts/repo-setup.sh to apply."
fi
