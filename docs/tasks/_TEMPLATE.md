---
id: 000
title: Short imperative title
milestone: M0
status: todo
deps: []
touches: [server/src/example/]
spec: ["4. Features"]
---

## Goal
One or two sentences: what exists after this task that didn't before.

## Acceptance criteria
- [ ] `cargo test -p polykov-server example::` passes, with tests for X and for the failure path Y
- [ ] `curl localhost:7700/api/example` returns 200 with `{...}`
- [ ] (each criterion is a command, a test, or an observable behaviour)

## Out of scope
- Things a builder might be tempted to do, but that belong to other tasks.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
