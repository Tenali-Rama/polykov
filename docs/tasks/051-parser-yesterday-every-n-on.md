---
id: 051
title: Parse "yesterday" and "every N weeks on <weekday>" in smart add
milestone: M3
status: todo
deps: []
touches: [parse/src/rules/when.rs, parse/src/rules/recur.rs, parse/src/words.rs, parse/tests/rules.rs, parse/bench/gen.py, parse/bench/train.jsonl, parse/bench/cases.jsonl, parse/src/tagger/model.bin, parse/tests/bench.rs, parse/API.md, docs/models/nl-parser.md]
spec: ["Capture", "5. Intelligence (local models)"]
---

## Goal
M2 review backlog. Smart add understands `yesterday`, and `every 2 weeks on tuesday` (also `every week on mon, thu` and `every other week on friday`, default — owner may change). It understands them through both the rules matchers and the shipped tagger.

## Acceptance criteria
- [ ] Rules matchers:
  - `yesterday` resolves to today − 1 day.
  - `every N weeks on <weekday list>` gives `FREQ=WEEKLY;INTERVAL=N;BYDAY=..`, and `every other week on fri` gives `INTERVAL=2`.
  - The first date is the first listed weekday on or after today.
  - Unit tests in `parse/tests/rules.rs` include an invalid case (`every 0 weeks on tue`) left as title text.
- [ ] The bench data covers these forms: `gen.py` templates, regenerated `train.jsonl`, and new hand cases in `cases.jsonl`. `model.bin` is retrained with the documented command (ADR 0012), and for `default_parser()`:
  - `call bob yesterday` and `standup every 2 weeks on tuesday 9am` parse correctly in a test
  - the `parse/tests/bench.rs` exact-match gate still passes; raise its threshold if the new score is higher
- [ ] `docs/models/nl-parser.md` gets a dated history line with the new scores (old vs new exact %, p99 latency).
- [ ] `parse/API.md` updated. `cargo test -p polykov-parse` and `scripts/verify.sh` pass.

## Out of scope
- New candidates or model types. Other phrasing gaps not listed here.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
