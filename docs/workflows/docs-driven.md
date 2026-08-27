# Docs-driven workflow

**Kicks in when:** the task at hand is writing/updating a doc, runbook,
config, or research — not source code. Applies per-task, not per-project —
a `coding` project doing a pure docs pass still uses this workflow for that
task.

For projects that are primarily documentation with incidental scripting
(homelab/infra setups, runbooks, config repos) — code exists to support the
docs, not the other way around, so a TDD/spec cycle is the wrong shape.

1. **Draft** — write or update the doc describing the change (setup steps,
   config, decision) before touching any script.
2. **Review** — walk the draft with the user one section at a time, same
   discipline as `/grill-me`: state what's being decided, propose a default,
   wait for confirmation.
3. **Script if needed** — only write a script where the doc says "run this";
   keep it a thin executor of what the doc already specifies, not new logic
   the doc doesn't describe. No test suite required — a runnable
   `--check`/dry-run mode is enough verification for infra scripts.
4. **Commit doc + script together** — the doc is the source of truth; a
   script without the doc update that motivated it is incomplete.

## Rules specific to live/infra systems

- **Don't execute against the user's live systems on their behalf.** For
  anything that reaches toward real infra (SSH, an ops TUI, a tool that
  changes homelab/production state) — narrate a step-by-step walkthrough and
  let the user run it themselves, even for "just show me a sample." Reserve
  direct execution for things scoped to the repo/dev environment itself
  (tests, builds, linters).
- **Verify state changes independently — don't trust "saved" or "applied."**
  A GUI/CLI reporting success means the config was accepted, not that the
  value is correct or that the running process picked it up. After any
  change that's supposed to take effect live, check with an independent
  probe (`dig` against a real resolver, `openssl s_client` against the
  actual port, a freshly-pulled log) before declaring it done.
