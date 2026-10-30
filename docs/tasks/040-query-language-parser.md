---
id: 040
title: Add the view query language parser
milestone: M3
status: todo
deps: []
touches: [server/src/query/mod.rs, server/src/query/parse.rs, server/src/query/tests.rs, server/src/lib.rs+, server/API.md, docs/decisions/0015-query-language.md]
spec: ["Organization", "Tasks (GTD)"]
---

## Goal
A pure, DB-free parser turns a filter query such as `type:task priority:<=2 due:<7d tag:work/* -status:done` into a typed AST, with errors that name the position. Task 041 compiles that AST to SQL.

## Acceptance criteria
- [ ] `query::parse(&str) -> Result<Query, ParseError>`. `ParseError` has a message and a char offset. Terms are space-separated and ANDed. A leading `-` negates a term. `"double quotes"` group words and values with spaces.
- [ ] Field terms (default set — owner may change):
  - `type:<type>`: one of the object types
  - `tag:<tag>` exact; `tag:<tag>/*` that tag and everything under it
  - `status:open|done`
  - `priority:` with `1`–`4` and the operators `=` (implicit), `<`, `<=`, `>`, `>=`
  - `due:` (the deadline), `scheduled:`, `created:`, `updated:`, `completed:`, each with the same operators. A value is `today`, `tomorrow`, `yesterday`, `YYYY-MM-DD`, a relative offset `7d`/`-3d`/`2w`/`1m` from today, or `none` (field unset).
  - `project:` and `area:`: an id, or a title in quotes
  - `pinned:true|false` and `archived:true|false|any`; archived defaults to `false` when not given, as in the object list
- [ ] Comma lists mean OR within one field: `status:open,done`, `priority:1,2` (default — owner may change).
- [ ] Bare words and quoted phrases become text terms, matched later through search (039).
- [ ] Relative dates stay symbolic in the AST. They resolve against a `today` given at compile time, so the AST is testable without a clock.
- [ ] Unit tests in `server/src/query/tests.rs` cover:
  - the spec example parsed to the expected AST
  - every field and operator
  - negation, quotes and comma lists
  - errors with offsets: an unknown field, a bad operator for a field (`tag:<x`), a bad date, `priority:5`, an unclosed quote, an empty value
  - `cargo test -p polykov-server query::` passes
- [ ] `docs/decisions/0015-query-language.md` records the grammar, the field list and the date semantics. The grammar summary also goes in `server/API.md`.

## Out of scope
- SQL compilation and the HTTP endpoint (041). OR between different fields, and parentheses. Front-matter property fields (M4).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
