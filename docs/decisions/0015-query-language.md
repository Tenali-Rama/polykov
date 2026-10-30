# 0015 — View query language

- **Status**: accepted
- **Context**: Spec §3 "Organization" asks for saved views over a filter query language (e.g. `type:task priority:<=2 due:<7d tag:work/* -status:done`). Task 040 parses it; 041 compiles it to SQL.
- **Decision**: `query::parse` (pure, no DB, no clock) reads whitespace-separated terms, all ANDed. `-` before a term negates it. `"..."` keeps spaces, `:` and `,` literal. A term with an unquoted `:` is `field:value[,value..]`, and the commas mean OR within that field. Any other term is a text term for search (039). Fields:
  - `type` (an object type), `tag` (`t` exact, `t/*` that tag and every tag under it), `status:open|done`.
  - `priority` 1–4 and the dates `due` (deadline), `scheduled`, `created`, `updated`, `completed`, each item taking `=` (implicit), `<`, `<=`, `>`, `>=`. Other fields take no operator.
  - A date is `today`, `tomorrow`, `yesterday`, `YYYY-MM-DD`, `[+-]N(d|w|m)` from today, or `none` (unset, `=` only).
  - `project`/`area`: an id, or a `"quoted title"`. `pinned:true|false`.
  - `archived:true|false|any` sets `Query.archived` (default `false`, as in the object list). It is not a term, may appear once, and `-archived:any` is an error.
  - Field names are lower case. Keyword values ignore case. An unknown field is an error, so quote text that contains `:`.
  - Relative dates stay symbolic (`DateValue::Relative(n, unit)`; today = `0d`). `DateValue::resolve(today)` turns one into a date, and a month offset clamps to the month's end.
- **Consequences**: Errors carry the char offset of the bad part. OR across fields, parentheses and property fields (M4) are not in the grammar. Adding one later must keep every query that parses today meaning the same.
