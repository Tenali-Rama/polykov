# 0009 — Smart-add parser crate

- **Status**: accepted
- **Context**: Smart add needs an NL parser (spec §4 Capture), and spec §5 asks for every model behind a stable Rust trait, compared against a simple baseline.
- **Decision**:
  - A pure workspace crate `parse/` (`polykov-parse`): no I/O, no clock, so candidates can be benchmarked and swapped. Date math uses `chrono` (no default features, `std` only).
  - `trait Parser { fn parse(&self, &str, &Context) -> Parsed }`. Output is shared with the server: dates in the 025 formats, rules in the 027 RRULE subset, tokens with byte offsets.
  - `RulesParser` is the hand-written baseline. Its syntax is the table in `parse/API.md` (default — owner may change): sigils `# + @` need a letter, the first value of a field wins, quoted text is never parsed, and a recurrence with no date schedules its first occurrence. Task 051 adds `yesterday` and `every N weeks on <weekdays>` (`every other week on fri` = interval 2); an invalid count there leaves the whole phrase as title text.
- **Consequences**: New candidates (030) implement `Parser` and reuse these types; the server depends only on the trait. Don't add I/O or a clock to the crate, and don't change the output formats without the server.
