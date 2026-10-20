#!/bin/sh
# Brings this project's GitHub repo onto the setup its profile records (.claude/profile.yaml:
# `branching`, `default_branch`, `components`, `required_checks` — see docs/gitflow.md).
# Idempotent — safe to re-run; each step is skipped when already in place:
#
#   1. gitflow: `develop` exists (created from origin/<default> if missing); trunk: n/a
#   2. delete-branch-on-merge is on
#   3. gitflow: the default branch is protected by the merge guard (required main-merge-guard
#      check, PRs only, admins included) — only once .github/workflows/main-merge-guard.yml is on
#      origin/<default>; before that a hotfix would wait forever on a check that never runs.
#      trunk: the default branch requires PRs and the profile's `required_checks`.
#   4. the `chore` issue label exists (CLAUDE.md "Issue Tracking" uses it; not a GitHub default)
#
# Components recorded `false` (integration-branch, merge-guard) skip their step.
#
#   scripts/repo-setup.sh --check   # report only, changes nothing
#   scripts/repo-setup.sh           # apply what's missing (changes GitHub — ask first)
#
# GitHub only: any other origin (Azure DevOps, GitLab, none) is skipped.
set -eu

MODE="${1:-apply}"
case "$MODE" in apply|--check) ;; *) echo "usage: repo-setup.sh [--check]" >&2; exit 1 ;; esac

pg() { sh "$(dirname "$0")/profile-get.sh" "$@"; }
branching="$(pg branching gitflow)"
default="$(pg default_branch)"
echo "repo-setup: branching=$branching default=${default:-(detect)}"

url="$(git remote get-url origin 2>/dev/null)" || { echo "repo-setup: no origin remote — skipped."; exit 0; }
case "$url" in
  *github.com*) ;;
  *) echo "repo-setup: origin is not GitHub ($url) — skipped (GitHub-only)."; exit 0 ;;
esac
command -v gh >/dev/null 2>&1 || { echo "repo-setup: gh CLI not found" >&2; exit 1; }

repo="$(gh repo view --json nameWithOwner -q .nameWithOwner)"
[ -n "$default" ] || default="$(gh repo view --json defaultBranchRef -q .defaultBranchRef.name 2>/dev/null || true)"
default="${default:-main}"
git fetch -q origin
pending=0
say() { printf '  %-22s %s\n' "$1" "$2"; }
off() { [ "$(pg "components.$1" true)" = false ]; }
echo "repo-setup: $repo ($MODE, $branching, default branch $default)"

# 1. develop
if [ "$branching" = trunk ]; then
  say develop "n/a (trunk)"
elif off integration-branch; then
  say develop "skipped (component off)"
elif git ls-remote --exit-code --heads origin develop >/dev/null 2>&1; then
  say develop "ok"
elif [ "$MODE" = --check ]; then
  say develop "MISSING — would create from origin/$default"; pending=1
else
  git push -q origin "refs/remotes/origin/$default:refs/heads/develop"
  say develop "created from origin/$default"
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

# 3. protection on the default branch
prot="$(gh api "repos/$repo/branches/$default/protection" --jq '[(.required_status_checks.contexts // [] | join(",")), (.required_pull_request_reviews != null)] | join(" ")' 2>&1)" || true
unavailable() { case "$prot" in *"Upgrade to GitHub Pro"*|*"HTTP 403"*) return 0 ;; esac; return 1; }
protect() {  # $1 = JSON array of required check names
  gh api -X PUT "repos/$repo/branches/$default/protection" --input - >/dev/null <<JSON
{
  "required_status_checks": { "strict": false, "contexts": $1 },
  "enforce_admins": true,
  "required_pull_request_reviews": { "required_approving_review_count": 0 },
  "restrictions": null
}
JSON
}
# *ARRgh: under gitflow the default branch also requires the profile's required_checks, not
# just the merge guard (template enforces required_checks for trunk only).
checks="$(pg required_checks)"
[ "$branching" = trunk ] || checks="main-merge-guard
$checks"
json="[$(printf '%s\n' "$checks" | sed '/^$/d; s/.*/"&"/' | paste -sd, -)]"
missing=""
# one check per line — names may contain spaces ("shellcheck + smoke test")
while IFS= read -r c; do
  [ -n "$c" ] || continue
  case ",${prot% *}," in *",$c,"*) ;; *) missing="$missing [$c]" ;; esac
done <<EOF
$checks
EOF
if [ "$branching" = trunk ]; then
  if unavailable; then
    say protection "unavailable — plan doesn't support branch protection on this repo"
  elif [ "${prot##* }" = true ] && [ -z "$missing" ]; then
    say protection "ok (PRs required)"
  elif [ "$MODE" = --check ]; then
    say protection "OFF — would require PRs + checks $json on $default"; pending=1
  else
    protect "$json"
    say protection "$default protected (PRs + checks $json)"
  fi
elif off merge-guard; then
  say merge-guard "skipped (component off)"
elif [ -z "$missing" ]; then
  say merge-guard "ok"
elif unavailable; then
  say merge-guard "unavailable — plan doesn't support branch protection on this repo"
elif ! git cat-file -e "origin/$default:.github/workflows/main-merge-guard.yml" 2>/dev/null; then
  say merge-guard "PENDING — workflow not on $default yet; re-run after the next release"; pending=1
elif [ "$MODE" = --check ]; then
  say merge-guard "OFF — would protect $default (checks $json)"; pending=1
else
  protect "$json"
  say merge-guard "$default protected (checks $json)"
fi

# 4. chore label
if gh label list -R "$repo" --json name -q '.[].name' | grep -qx chore; then
  say chore-label "ok"
elif [ "$MODE" = --check ]; then
  say chore-label "MISSING — would create"; pending=1
else
  gh label create chore -R "$repo" --color c5def5 --description "Maintenance, infra, CI, deps — no user-facing change" >/dev/null
  say chore-label "created"
fi

if [ "$MODE" = --check ] && [ "$pending" -eq 1 ]; then
  echo "repo-setup: changes pending — run scripts/repo-setup.sh to apply."
fi
