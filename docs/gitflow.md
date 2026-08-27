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
- **`hotfix/<name>`** — urgent production fix. Same flow; expedited review, tag a patch release immediately after merge.

No `develop`, no `release/*` branches. Release prep happens on a normal
`chore/` or `release/` topic branch and merges to `main` like anything else.

## Topic flow

```bash
git checkout main && git pull
git checkout -b feature/<name>
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
git checkout -b chore/release-vX.Y.Z main
# bump <Version> in server/ArrghServer.csproj, add docs/releases/vX.Y.Z.md,
# add CHANGELOG.md row
git push -u origin chore/release-vX.Y.Z
gh pr create --base main --title "release vX.Y.Z"
# after merge:
git checkout main && git pull
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

Version source of truth is `server/ArrghServer.csproj` `<Version>` —
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
