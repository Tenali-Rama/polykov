---
id: 073
title: Add toggleable hover and inline previews for internal and external links in notes
milestone: M4
status: todo
deps: [070]
touches: [web/src/notes/Preview.svelte, web/src/notes/Preview.test.ts, web/src/notes/preview.ts, web/src/notes/preview.test.ts, web/src/notes/NotePage.svelte+, web/e2e/link-previews.spec.ts, web/API.md, docs/decisions/0023-link-previews.md]
spec: ["Notes", "1. Principles"]
---

## Goal
Hovering or focusing a link in a note previews its target. The owner can switch previews off, to hover, or to inline.

## Acceptance criteria
- [ ] Internal targets (resolved wikilinks and `#/o|n|v|p/<id>` links):
  - a note shows its type, title, tags and the first 300 characters of its body as plain text
  - a task shows its status, priority and dates
  - anything else shows its type, title and tags
- [ ] External `http(s)` links show the full URL and the domain only. Nothing is fetched from the outside, server or client, per spec "no third-party data calls" (default — owner may change).
- [ ] Modes, chosen with a `previews: off|hover|inline` select in `NotePage` panels (one line) and stored in `localStorage`, default `hover`:
  - `hover`: a box-bordered popover after 300 ms of hover, or right away on keyboard focus, closed by `Esc` (handled) or moving away. It stays inside the viewport and is linked with `aria-describedby`.
  - `inline`: `Enter` on a focused link, or a tap on phones, expands the preview under its paragraph until it's collapsed.
- [ ] Previews are cached per target for the page's lifetime. A trashed or missing target shows `not found`.
- [ ] Unit tests: `preview.ts` (target from href, cache) and `Preview.svelte` (each target kind, external, not found, each mode).
- [ ] `web/e2e/link-previews.spec.ts`: keyboard focus on a wikilink to a note shows its body excerpt. Switch to `inline`, `Enter` expands it, then switch to `off` and nothing shows. An external link shows its domain, and the test asserts no request leaves the origin. On `phone`, `expectPhoneFit` passes with the popover open.
- [ ] ADR `docs/decisions/0023-link-previews.md` (≤ 15 lines): no external fetches, and the modes. `web/API.md` updated. `scripts/verify.sh` passes.

## Out of scope
- Previews outside notes. Fetching page titles of external sites.

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
