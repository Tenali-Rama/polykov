---
id: 030
title: Run the NL parser experiment and record the winner in docs/models
milestone: M2
status: todo
deps: [029]
touches: [parse/bench/, parse/examples/bench.rs, parse/tests/bench.rs, parse/src/, parse/Cargo.toml, parse/API.md, Cargo.lock, docs/models/nl-parser.md]
spec: ["5. Intelligence", "Capture"]
---

## Goal
The parser that smart add ships is chosen by a benchmark, as spec §5 requires. The results live in `docs/models/nl-parser.md`.

## Acceptance criteria
- [ ] Benchmark set `parse/bench/cases.jsonl`. Each line is `{input, today, projects, areas, expected:{title, scheduled, deadline, estimate_min, priority, recurrence, project, area, tags}}`. It has at least 300 cases:
  - at least 60 hand-written casual phrases (abbreviations, odd word order, no-token titles, false-positive traps)
  - the rest synthetic, from templates
  - a "cold start" slice with no known projects or areas, and a "long history" slice with 50 known projects/areas and 100 tags
  - Each case is tagged with its slice. No real personal data.
- [ ] At least three candidates behind `Parser`:
  - the 029 `RulesParser` (baseline)
  - rules with dates from an existing date-phrase crate
  - a small learned token tagger (e.g. an averaged perceptron trained on a disjoint synthetic split) feeding the same value normalizers
  - CPU only, no network, and model RAM well under the spec §5 1 GB total
- [ ] `cargo run -p polykov-parse --release --example bench` prints, for each candidate and slice:
  - whole-parse exact match
  - per-field accuracy
  - false-token rate on no-token titles
  - p50/p99 latency
  - peak RAM and model size
- [ ] `docs/models/nl-parser.md` follows `docs/models/README.md`: targets, benchmark, candidates, results table, decision, re-run log.
  - The winner is the most accurate candidate with p99 < 20 ms.
  - An ML candidate ships only if it beats the baseline.
  - `polykov_parse::default_parser() -> Box<dyn Parser>` returns the winner.
- [ ] `parse/tests/bench.rs` runs `default_parser()` over the benchmark and fails if whole-parse exact match drops below the recorded score minus 1 point. It passes under `cargo test -p polykov-parse`.
- [ ] Losing candidates that don't ship are deleted from the code, not left behind, and are described in the doc. The baseline stays.

## Out of scope
- Learning from the user's own corrections at runtime (the tagger is M11). HTTP endpoints (031).

## Notes for next
<!-- builder fills in -->
