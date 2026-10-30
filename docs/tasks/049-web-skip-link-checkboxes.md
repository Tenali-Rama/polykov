---
id: 049
title: Add a skip link to the command line and uniform 44 px checkboxes
milestone: M3
status: todo
deps: []
touches: [web/src/Shell.svelte+, web/src/Shell.test.ts, web/src/app.css, web/src/objects/List.svelte, web/e2e/a11y-fixes.spec.ts, web/API.md]
spec: ["7. Clients & UI", "8. Customization & Accessibility"]
---

## Goal
M2 review backlog. The bottom command line is reachable from the keyboard in a few presses, and every checkbox has the same 44 px touch target. This task adds no features.

## Acceptance criteria
- [ ] `Shell.svelte` only gains lines: a visually hidden link `skip to command line` as the first focusable element, which shows when focused and moves focus to `#command`. Its CSS is appended in the same file.
  - The first `Tab` after load focuses it, and `Enter` focuses `#command`.
  - A Vitest test in `Shell.test.ts` covers this.
- [ ] One shared rule in `app.css` sizes every `input[type=checkbox]` (box itself ≥ 44 × 44 px, focus outline visible, colours from the CSS variables). The local rule in `objects/List.svelte` is removed.
- [ ] `web/e2e/a11y-fixes.spec.ts`:
  - On `desktop` and `phone`, from page load with 30 seeded projects, `#command` is focused within 2 key presses (`Tab`, `Enter`).
  - Every checkbox on the object list measures ≥ 44 × 44 px on `phone`, and `expectPhoneFit` passes.
- [ ] `web/API.md` updated if it documents Shell. `npm --prefix web run test` and `scripts/e2e.sh` pass.

## Out of scope
- Collapsing projects/areas in the nav. Wiring the command line to the palette (044 question).

## Notes for next
<!-- builder fills in: ≤ 5 bullets that a later task needs to know -->
