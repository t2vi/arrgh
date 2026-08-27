# Coding workflow

**Kicks in when:** the task at hand is source-code changes — new/modified
functions, a script beyond a thin executor, anything with a test suite to
extend. Applies per-task, not per-project — a `docs-driven` project still
uses this workflow for the parts of a task that are actual code.

For projects primarily driven by writing and changing code.

Spec-first flow before code: `/grill-with-docs` → implement with `/tdd`. Use
`/to-spec`/`/to-tickets` instead when the work is already broken into a plan
or slices — that's the exception, not the default first step. Skip the spec
step only if explicitly told to for that request.

Bug fixes and small changes that don't need a spec still go through `/tdd`
where the codebase has a test suite to extend — except a literal config-only
change with no testable logic (a version bump, an env var value), which
skips the test but not the review that motivated it.
