---
id: 052
title: Make CLI integration tests stable under load with longer login and startup timeouts
milestone: M3
status: todo
deps: []
touches: [cli/src/client.rs, cli/tests/common/mod.rs, cli/API.md]
spec: ["10. Non-Functional"]
---

## Goal
M2 review backlog. CLI integration tests failed three times under load: `login` (password hashing on the server) and the test server's start-up hit fixed timeouts. They become stable.

## Acceptance criteria
- [ ] In `cli/src/client.rs`, requests that hash a password (`login`, and `setup` if the CLI sends it) use a longer timeout of 60 s (default — owner may change). Other requests keep 10 s. One named constant per timeout, documented in `cli/API.md`.
- [ ] In `cli/tests/common/mod.rs`, `Server::start` waits until the server answers `/health`, retrying for up to 30 s, before calling `/api/setup`. The setup call uses a 60 s timeout. A failure panics with a message naming the step.
- [ ] Under load, `for i in 1 2 3; do taskset -c 0 cargo test -p polykov --quiet -- --test-threads=16 || exit 1; done` passes, i.e. three runs pinned to one CPU. The PR includes the timings.
- [ ] `scripts/verify.sh` passes.

## Out of scope
- Changing the server's password hashing cost. Retrying failed requests in the CLI itself.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
