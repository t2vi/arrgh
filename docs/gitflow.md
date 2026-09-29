# Git Workflow

*ARRgh uses a single-trunk model — **not** full gitflow. One long-lived
branch (`main`), short-lived topic branches, PRs merge back into `main`.
(OAIKit's template ships a `develop`-based gitflow; this project
deliberately diverges — keep it that way unless the team decides otherwise.)

## Branches

- **`main`** — always releasable. Protected: changes land via PR, not direct push. Release tags (`vX.Y.Z`) live here.
- **`feature/<name>`** — a new capability. Branch from `main`, PR back into `main`. Delete after merge.
- **`bugfix/<name>`** — a fix for something already on `main`. Same flow.
- **`chore/<name>`** — maintenance, infra, CI, deps — no user-facing change. Same flow.
- **`release/vX.Y.Z`** — release prep only: version bump, `CHANGELOG.md`, `docs/releases/vX.Y.Z.md`. Branch from `main`, PR back into `main`, tag after merge. Never bump the version or write release notes inline on a `feature/`/`bugfix/`/`chore/` branch — even when the release is really just shipping one already-finished fix, cut a `release/` branch for that step.
- **`hotfix/<name>`** — urgent production fix. Same flow; expedited review, tag a patch release immediately after merge.

No `develop` branch. Every topic branch above targets `main` directly.

Always branch out before starting work — never keep editing on `main`, and don't reuse a
branch left over from a different, unrelated prior task; cut a fresh one named for the task
actually at hand.

## Topic flow

```bash
git checkout main && git pull
git checkout -b feature/<name>   # or bugfix/<name>, chore/<name> — same flow
# ...work, commit...
git push -u origin feature/<name>
gh pr create --base main
# after merge:
git branch -d feature/<name>
```

## Cutting a release

Follow the **Feature-Ready Checklist** in `CLAUDE.md` (docs, build, tests,
e2e, Docker, version bump, release notes). Then:

```bash
git checkout -b release/vX.Y.Z main
# bump [package] version in server/Cargo.toml, add docs/releases/vX.Y.Z.md,
# add CHANGELOG.md row
git push -u origin release/vX.Y.Z
gh pr create --base main --title "release vX.Y.Z"
# after merge:
git checkout main && git pull
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

Version source of truth is `server/Cargo.toml`'s `[package] version` —
`GET /api/version` reads it at runtime, the UI shows it dynamically. There
is no root `VERSION` file (OAIKit's template uses one; *ARRgh does not).

CI publishes on push to `main` / tag push — see `.github/workflows/`.

## Private overlay

The AI/dev files (`CLAUDE.md`, `CONTEXT.md`, `.claude/`, `docs/adr/`,
`docs/agents/`, `graphify-out/`, `api-live-tests/`, `live-tests/`,
`.agents/`, `.understand-anything/`, `skills-lock.json`) are **not** in this
public repo's history. They live on disk, tracked by a separate overlay
git-dir (`.private.git/`) that pushes to the private mirror
`t2vi/arrgh-dev`. Manage with `scripts/private-overlay.sh {status|commit|push|pull}`.
Config: `.claude/private-overlay.conf`. See `CLAUDE.md` → Git Workflow.

## Claude Code sessions

Interactive and background sessions both work directly on the checked-out
branch — no automatic worktree isolation (`.claude/settings*.json` sets
`worktree.bgIsolation: none`).

## Recovering from an amend after push

If a commit already pushed to a remote branch gets `--amend`ed locally, the
next `push` is rejected (non-fast-forward) — local and remote copies of that
commit have diverged. Don't force-push to "fix" it unless the amend
intentionally needs to overwrite what's public; usually the amend just
folded in one more change that should've been its own commit. Undo it
instead — no force-push needed:

```bash
git log --oneline -1 origin/<branch>   # the commit that's actually public
git reset --soft <that-commit-sha>     # rewinds HEAD to it; working tree/index untouched
git commit -m "the extra change"       # what got folded in via amend, now its own commit
git push                                # fast-forwards cleanly — no --force
```
