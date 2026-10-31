---
id: 065
title: Add the web note page with a TipTap editor that round-trips markdown
milestone: M4
status: todo
deps: [062]
touches: [web/package.json, web/package-lock.json, web/src/lib/notes.ts, web/src/lib/notes.test.ts, web/src/notes/route.ts, web/src/notes/route.test.ts, web/src/notes/NotePage.svelte, web/src/notes/NotePage.test.ts, web/src/notes/Editor.svelte, web/src/notes/markdown.ts, web/src/notes/markdown.test.ts, web/src/notes/extensions/index.ts, web/src/objects/Detail.svelte, web/src/objects/Detail.test.ts, web/src/Shell.svelte+, web/e2e/notes.spec.ts, web/CLAUDE.md, web/API.md, docs/decisions/0021-note-editor.md]
spec: ["Notes", "7. Clients & UI"]
---

## Goal
`#/n/<id>` opens a note in a block editor. The owner edits the title, the front-matter properties and the body, and the body saves as markdown that reads back unchanged.

## Acceptance criteria
- [ ] Add TipTap (`@tiptap/core`, `@tiptap/starter-kit`, plus a markdown serializer such as `tiptap-markdown`; the builder picks and records the choice) to `web/package.json`. Build on vanilla `@tiptap/core` in a Svelte 5 component. ADR `docs/decisions/0021-note-editor.md` (≤ 15 lines) records the editor, that markdown is the stored format, and the extension registry.
- [ ] Extension registry: `web/src/notes/extensions/index.ts` exports the editor's extension list, one entry per line. Later tasks (067–073) each add one line there. Put that rule in `web/CLAUDE.md`.
- [ ] `lib/notes.ts`: `getNote`, `createNote`, `updateNote`, `listNotes`, `listNotebooks`, `createNotebook`, `updateNotebook`, with unit tests. `notes/route.ts`: `noteHref(id)` (`#/n/<id>`), `notesHref` (`#/notes`), `notebookHref(id)` (`#/nb/<id>`) and `parseNoteHash`, with tests. Shell gains only its `#/n/<id>` branch and import (one appended block).
- [ ] `NotePage`:
  - The title edits in place.
  - The properties show as a front-matter block (`---` / `key: value` / `---`) above the body, edited with the existing `PropsEditor`.
  - The body is the editor. It autosaves 1 s after the last change and on `Ctrl+S`, shows `saved`/`saving…`/`error: <cause>`, and never loses unsaved text on Esc. After `Detail`, it has a `panels` area where later tasks add one line each.
  - `Detail` of a `note` shows a link `[ open note ]` to `#/n/<id>`.
- [ ] Markdown round-trip: `markdown.test.ts` loads fixtures for headings, paragraphs, bold/italic/strike/inline code, lists (nested, ordered), blockquote, code block, horizontal rule and hard break. For each one, `serialize(parse(md)) === md` after normalising the fixture once. Typing in the editor and reloading gives the same text.
- [ ] `web/e2e/notes.spec.ts`, against the real server: keyboard only, create a note through `api()`, open `#/n/<id>`, type a heading and a list, wait for `saved`, reload, and the content and `GET /api/notes/{id}` body match. On `phone`, `expectPhoneFit` passes, and the editor is full width with no horizontal scroll.
- [ ] `web/API.md` updated. `scripts/verify.sh` passes. The PR states the JS bundle size before and after.

## Out of scope
- The notebooks browser, nav and palette entries (066). Tables, code highlighting, math, mermaid (067). Blocks (068, 069). Wikilinks (070).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
