# Content workflow

**Kicks in when:** the task at hand is writing publishable content — an
article, blog post, or public-facing copy — where voice and structure
matter more than correctness or infra state. Applies per-task, not
per-project — a `coding` project's occasional blog post still uses this
workflow for that task.

For projects that are primarily authored content (a personal site/blog).

1. **Voice/structure first** — follow this project's own voice guide and
   structure conventions, wherever they're documented (`CLAUDE.md`/
   `CONTEXT.md`). Don't invent new structure or tone per post; this workflow
   doesn't define the voice itself, the project does.
2. **Scope check** — before drafting, confirm what's off-limits to write
   about (work repos, client work, anything the project's scope rules
   exclude) and how much detail an in-scope topic gets.
3. **Draft → review** — same discipline as `/grill-me`: draft, walk it with
   the user section by section, wait for confirmation before publishing.
4. **No test/build gate on the writing itself** — content is done when the
   user approves it, not when a build passes. If the site has its own build
   step, that's a `coding`-workflow concern for the site's tooling, separate
   from the writing.
