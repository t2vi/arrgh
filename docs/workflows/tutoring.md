# Tutoring workflow

**Kicks in when:** the task at hand is teaching a skill through structured
exercises — not shipping product code. Inverts `coding`'s TDD roles: Claude
writes the exercise and its test, the learner writes the implementation.

For projects whose purpose is the user learning something (a language, a
technique), not the code itself.

1. **Curriculum first** — before the first exercise, lay out the topic
   sequence and confirm it with the learner. Keep it as a checklist in-repo
   (e.g. `curriculum/progress.md`) so a new session resumes at the right
   point instead of re-asking where things left off.
2. **From scratch, regardless of general experience** — cover the language's
   own fundamentals in full (syntax, idioms, tooling) even when the learner
   is an experienced developer in other languages. General programming
   background doesn't excuse skipping steps or compressing the curriculum —
   this language's idioms (e.g. Go's error handling, interfaces, goroutines)
   still need dedicated exercises, not "you already get this."
3. **Exercise = problem + test, no solution** — write a problem statement
   and a test file that defines "done." Never write the implementation,
   even on request ("just show me") — that defeats the exercise.
4. **Learner implements** — the learner writes the code against the test,
   not Claude.
5. **Hint, don't fix, on failure** — a failing test gets a hint pointing at
   the concept or failing behavior, not the diff or the fix. Escalate hint
   specificity only after repeated struggle on the same exercise.
6. **Advance only on green** — move to the next exercise only once the
   current one's tests pass; a brief "why did that work" check before
   advancing catches pattern-matching without understanding.
7. **Track progress in-repo** — update the curriculum checklist each
   session so state survives between sessions without relying on memory.
