# parse

`polykov-parse`: the smart-add natural-language parser. Spec §4 Capture, §5 Intelligence.

## Layout
- `src/lib.rs`: the `Parser` trait and its types (`Context`, `Parsed`, `Token`, `Kind`, `Recurrence`, `Anchor`).
- `src/words.rs`: splits a line into words with byte offsets and protects `"quoted text"`.
- `src/rules/`: `RulesParser`, the hand-written baseline. `mod.rs` drives the left-to-right match; `when.rs` dates and times, `recur/` recurrence (`mod.rs` phrases, `weekday.rs` weekday lists, `rule.rs` the RRULE), `marks.rs` tags, projects, areas, priority and durations.
- `src/tagger/`: `TaggerParser`, the shipped winner (`default_parser()`). `features.rs` word features; `model.bin` the bundled model, written by the bench's `--save-model`.
- `bench/`: the benchmark. `gen.py` (+ `hand.py`) writes `cases.jsonl` and the training split `train.jsonl`; `score.rs` loads and scores cases, shared by `examples/bench.rs` (the experiment) and `tests/bench.rs` (the regression gate).
- `tests/rules.rs`: whole-line behaviour, including the spec example and plain-title traps.

## Rules
- Pure: no I/O, no network, no clock. "Today" always comes from `Context`.
- Every candidate parser implements `Parser`; callers depend only on the trait.
- Output formats are shared with the server: dates as in task 025, rules in the 027 RRULE subset. Change them only together.
- A syntax change updates `API.md` and ADR 0009, and gets a test.
- A change to rules, features or training re-runs the bench, retrains `model.bin` and updates `docs/models/nl-parser.md` and `RECORDED_EXACT` in `tests/bench.rs`.
