---
id: 056
title: Parse bare "every other week" and split the recurrence rules file
milestone: M4
status: todo
deps: []
touches: [parse/src/rules/recur.rs, parse/src/rules/recur/, parse/tests/rules.rs, parse/bench/cases.jsonl, parse/tests/bench.rs, parse/API.md, docs/models/nl-parser.md]
spec: ["Capture", "5. Intelligence (local models)"]
---

## Goal
Smart add reads "every other week" without "on <day>" as a 2-week rule, and the recurrence rules live in files under 400 lines.

## Acceptance criteria
- [ ] `parse/src/rules/recur.rs` becomes a module dir `parse/src/rules/recur/` (e.g. `mod.rs` + `interval.rs` + `weekday.rs`), each file ≤ 300 lines. The `rules/mod.rs` `mod recur;` line still works unchanged, and public behaviour is the same: every existing test in `parse/tests/rules.rs` passes unchanged.
- [ ] `"water plants every other week"` gives the same rule as `every 2 weeks` (anchored to the start date, as that phrase is today), with the title `water plants`. `"every other week on fri"` is unchanged. Each case has a test in `parse/tests/rules.rs`.
- [ ] `"every other"` with nothing after it isn't a rule: it stays in the title. There's a test for this.
- [ ] Add the new phrase to `parse/bench/cases.jsonl`. `cargo test -p polykov-parse --test bench` passes, and the accuracy in `docs/models/nl-parser.md` doesn't drop (update the table if it rises).
- [ ] `scripts/verify.sh` passes.

## Out of scope
- Retraining the tagger. Other new recurrence phrases.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
