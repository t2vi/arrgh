# QA workflow (functional test planning)

**Kicks in when:** the task at hand is turning requirements into structured
functional test cases / test plans — not test code (that's `coding`'s test
suite) and not narrative documentation (that's `docs-driven`).

For projects whose output is manual/functional test case documentation
covering a system's behavior — no application code lives here.

1. **Confirm the target platform first** — Azure DevOps Test Plans,
   TestRail, Xray/Jira, or plain Gherkin/markdown if nothing's decided yet.
   Record the choice in this project's `CLAUDE.md` so later sessions don't
   re-ask. Every format decision below follows from this choice.
2. **Requirements → coverage plan before test cases** — map each
   requirement (feature ref, PBI, ticket) to the scenarios it needs: actor/
   role variants, partial-vs-complete state, path/allocation variants,
   cross-system boundaries. Confirm the coverage list with the user before
   drafting full steps — same discipline as `/grill-me`.
3. **One test case, one file** — named/prefixed to the requirement (e.g.
   `FR1919-TC01-<slug>.md`). Steps are `Action` + `Expected Result`; prefix
   the actor when switching roles (`As a <role>, ...` / `Login as <role>`),
   `Logout` as its own step on actor switches, blank `Expected Result` only
   for pure setup/navigation steps.
4. **API-level cases get their own shape** — flag it at the top of the
   file; steps describe the request payload and expected response instead
   of a UI interaction.
5. **MD is the source of truth, always paired with a platform-native
   export** — once a requirement's MD test cases are done, generate the
   bulk-import artifact for the confirmed platform (e.g. an Azure DevOps
   Test Plans CSV) alongside them. Don't skip the export because "the MD is
   enough" — the MD is for source control, the export is what actually
   loads into the tool.
6. **Mirror the platform's own hierarchy in the repo layout** — directory
   structure follows however the target platform groups things (iteration/
   sprint, area path, folder, suite), so the repo maps 1:1 onto where test
   cases land in the tool.
