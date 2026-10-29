# parse

`polykov-parse`: the smart-add natural-language parser. Spec §4 Capture, §5 Intelligence.

## Layout
- `src/lib.rs`: the `Parser` trait and its types (`Context`, `Parsed`, `Token`, `Kind`, `Recurrence`, `Anchor`).
- `src/words.rs`: splits a line into words with byte offsets and protects `"quoted text"`.
- `src/rules/`: `RulesParser`, the hand-written baseline. `mod.rs` drives the left-to-right match; `when.rs` dates and times, `recur.rs` recurrence, `marks.rs` tags, projects, areas, priority and durations.
- `tests/rules.rs`: whole-line behaviour, including the spec example and plain-title traps.

## Rules
- Pure: no I/O, no network, no clock. "Today" always comes from `Context`.
- Every candidate parser implements `Parser`; callers depend only on the trait.
- Output formats are shared with the server: dates as in task 025, rules in the 027 RRULE subset. Change them only together.
- A syntax change updates `API.md` and ADR 0009, and gets a test.
