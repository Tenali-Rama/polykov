# Quality Bar

Builders meet this before opening a PR. Reviewers check every point. A PR merges only when there are no blocking findings.

## Definition of done
1. **Acceptance criteria**: every criterion is met. The reviewer re-checks each one and does not trust the PR's evidence table.
2. **Verify**: `scripts/factory/clean-verify.sh` passes on the pushed branch, and CI is green.
3. **Tests**:
   - New behaviour is covered, including at least one edge or failure path.
   - Tests would fail without the change.
   - No tests that only assert mocks.
4. **Spec**: matches the referenced spec sections. Nothing is invented beyond them.
5. **Scope**: changes stay within the task's `touches`, or the PR says why not. No drive-by refactors.
6. **Code**:
   - Reads like the surrounding code.
   - Files ≤ 400 lines.
   - No dead code. No TODO without a task id.
   - No `unwrap`/`expect` in non-test server code.
   - Errors are handled and give useful messages.
7. **Docs**: the task file's Notes are filled in. `API.md` is updated if an interface changed. A lasting decision gets an ADR.
8. **Security**:
   - No secrets in the code or tests.
   - Input is validated at the API boundary.
   - Auth is required on every non-public endpoint.
   - No data leaves the server (spec §1).
9. **Performance**: respects the budgets in spec §1/§10 where they apply.
10. **UI**:
    - Minimal, CLI-look style (spec §7–8).
    - Every action can be done by keyboard.
    - Works at phone width.
    - Touch targets ≥ 44px on Android.
11. **ML**: there is an experiment log in `docs/models/` that compares against a baseline (spec §5).
12. **Git**: one task per PR. The owner is the author. No AI attribution.

## Review findings
- **Blocking**: anything that breaks 1–12. Write each one as specific and actionable: `file:line`, what to change, and why.
- **Optional**: nits and suggestions. List them separately. They never block a merge.
- Tag every blocking finding with a short slug (e.g. `missing-edge-test`, `scope-creep`). The COO tracks repeat offenders.
