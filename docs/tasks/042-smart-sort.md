---
id: 042
title: Add smart sort and sort=smart on the GTD lists API
milestone: M3
status: todo
deps: []
touches: [server/src/smartsort/, server/src/lib.rs+, server/src/lists/mod.rs, server/src/lists/store.rs, server/tests/smartsort.rs, server/API.md, docs/decisions/0017-smart-sort.md]
spec: ["Tasks (GTD)"]
---

## Goal
Tasks can be ranked by a documented score built from priority, deadline proximity and estimate, with goal relevance reserved for when goals exist. The GTD lists offer it now, and saved views (043) reuse it.

## Acceptance criteria
- [ ] `smartsort::score(&Fields, today) -> f64` is pure and deterministic, and `smartsort::sort(&mut [Task], today)` uses it. Its inputs:
  - priority: P1 counts most
  - deadline proximity: overdue counts most, then closer deadlines; no deadline adds nothing
  - estimate: a short estimate ranks slightly higher (quick wins)
  - goal relevance: a term fixed at 0 until goals land in M9 (default — owner may change)
  - Ties break by `id`.
- [ ] The weights are constants in one place, and ADR `docs/decisions/0017-smart-sort.md` records the formula.
- [ ] Unit tests in `server/src/smartsort/` cover:
  - each factor alone changes the order in the expected direction
  - an overdue P3 vs a P1 due in 30 days, ranked as the ADR states
  - no deadline and no estimate
  - stability on ties
- [ ] `GET /api/lists/{name}?sort=smart` returns the list in smart order. Without `sort`, or with `sort=default`, the order is unchanged. Any other value gets `400 {"error":"sort must be default or smart"}`. `server/tests/smartsort.rs` checks a seeded Today list in both orders.
- [ ] `server/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Sort in saved views (043). UI for choosing the sort (045). Goal links (M9). Learned ranking (M12).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
