# parse API

Crate `polykov_parse`. Pure: no I/O, no network, no clock. See ADR 0009.

## Interface
- `trait Parser { fn parse(&self, input: &str, ctx: &Context) -> Parsed }`: never fails; anything not understood stays in the title.
- `Context { today: NaiveDate, projects: Vec<String>, areas: Vec<String> }`: the local date, and the known project and area names.
- `Parsed { title, tokens: Vec<Token>, scheduled, deadline: Option<String>, estimate_min: Option<u32>, priority: Option<u8>, recurrence: Option<Recurrence>, project, area: Option<String>, tags: Vec<String> }`:
  - `scheduled`/`deadline`: `YYYY-MM-DD`, or `YYYY-MM-DDTHH:MM` with a time (the 025 formats).
  - `estimate_min`: 1–10080. `priority`: 1–4.
  - `project`/`area`: the matching name from `Context`, as given there.
  - `tags`: normalized like the server's tags (no `#`, lowercase), deduplicated, in input order.
- `Token { start, end, kind: Kind, text }`: `start..end` are byte offsets into the input and `text` is that slice. Tokens are in input order.
- `Kind`: `Date Time Duration Priority Project Area Tag Deadline Recurrence`.
- `Recurrence { rule: String, from: Anchor }`, `Anchor::{Scheduled, Completion}`: `rule` is in the 027 RRULE subset (`FREQ=…[;INTERVAL=n][;BYDAY=..][;BYMONTHDAY=n]`, `INTERVAL` left out when 1).
- All output types derive `Serialize`; `Kind` and `Anchor` serialize lowercase (`"date"`, `"scheduled"`).
- `RulesParser`: unit struct, the rules baseline.

## `RulesParser` syntax
Words are matched left to right, ignoring case; a phrase is whole words. For a field given twice the first value wins, but every match is a token and leaves the title.

| Kind | Forms |
|---|---|
| Tag | `#tag`, `#a/b` (must contain a letter: `#1` is text) |
| Project / Area | `+name` / `@name`, matched against `Context` ignoring case, `-`/`_` = space. Unknown names are tokens with no value. Must contain a letter. |
| Priority | `p1`–`p4` |
| Duration | `30m`, `45min`, `2h`, `1h30m`, `1.5h` |
| Date | `today`/`tod`, `tomorrow`/`tmrw`/`tmr`, weekday names and 3-letter abbreviations (next such day, today included), `next week` (next Monday), `next month` (the 1st), `in N days\|weeks\|months`, `nov 12`/`12 nov`/`november 12th` (next year if past), `2026-11-12` |
| Time | `5pm`, `5:30pm`, `17:00`, `at 17`, `at 5pm` |
| Deadline | `due <date> [time]`, `by <date> [time]` |
| Recurrence | `every day\|weekday\|week\|month\|year`, `daily`…`yearly`, `every <weekday>[, and <weekday>…]`, `every N days\|weeks\|months\|years` (N 1–365), `every 15th`; `every!` counts from completion |

- `scheduled` = the date, else the recurrence's first occurrence on or after today, else today if a time was given; plus the time if any. A deadline never sets `scheduled`.
- `"quoted text"` is never parsed and a phrase never spans it; the paired quotes are dropped from the title. An unpaired `"` is plain text.
- `title` is the input minus tokens and paired quotes, whitespace collapsed.
