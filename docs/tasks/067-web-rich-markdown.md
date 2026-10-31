---
id: 067
title: Add GFM tables, task lists, highlighted code, math and mermaid to the note editor
milestone: M4
status: todo
deps: [065]
touches: [web/package.json, web/package-lock.json, web/src/notes/extensions/index.ts+, web/src/notes/extensions/tables.ts, web/src/notes/extensions/code.ts, web/src/notes/extensions/math.ts, web/src/notes/extensions/mermaid.ts, web/src/notes/extensions/rich.test.ts, web/e2e/notes-rich.spec.ts, web/API.md, docs/decisions/0026-note-markdown-dialect.md]
spec: ["Notes"]
---

## Goal
Notes support full GFM plus math and mermaid. Each renders in the editor and serialises back to the same markdown.

## Acceptance criteria
- [ ] Each piece is an extension file, registered with one appended line in `extensions/index.ts`:
  - GFM tables (add and remove rows and columns from the keyboard) and task lists `- [ ]`
  - fenced code with syntax highlighting (`lowlight`, common languages only)
  - math: inline `$…$` and block `$$…$$`, rendered with KaTeX
  - fenced ```` ```mermaid ```` blocks rendered as a diagram, with the source editable on focus
- [ ] Mermaid and KaTeX are lazy-loaded (dynamic `import()`): the main bundle grows by ≤ 150 KB gzip (state the numbers in the PR). A mermaid syntax error shows `mermaid: <message>` in the block and keeps the source.
- [ ] `rich.test.ts`: round-trip fixtures `serialize(parse(md)) === md` for a table with alignment, a task list, code with a language, inline and block math, and a mermaid block.
- [ ] Colors come only from `app.css` variables, including the highlight theme. It's readable in light and dark.
- [ ] `web/e2e/notes-rich.spec.ts`: a note seeded through the API with one of each renders (table cells, a `.katex` element, an SVG from mermaid). Edit a table cell from the keyboard, reload, and the change is kept. On `phone`, `expectPhoneFit` passes: wide tables and code scroll inside their own box.
- [ ] ADR `docs/decisions/0026-note-markdown-dialect.md` (≤ 15 lines): GFM plus `$`/`$$` math plus mermaid fences. `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Toggles, callouts, drag reorder (068). Embeds (069).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
