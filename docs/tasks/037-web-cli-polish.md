---
id: 037
title: Fix M0/M1 review nits in web focus, keys, setup and CLI edit
milestone: M2
status: done
deps: []
touches: [web/src/App.svelte, web/src/App.test.ts, web/src/Shell.svelte, web/src/Shell.test.ts, web/src/auth/Setup.svelte, web/src/objects/route.ts, web/src/objects/Detail.svelte, web/src/objects/Detail.test.ts, web/src/objects/List.svelte, web/src/objects/List.test.ts, web/src/objects/TagTree.svelte, web/src/objects/Links.svelte, web/src/objects/Links.test.ts, web/src/objects/Comments.svelte, web/src/objects/Comments.test.ts, web/API.md, cli/src/cmd/object/edit.rs, cli/src/client.rs, cli/tests/object.rs, cli/API.md]
spec: ["7. Clients & UI", "8. Customization & Accessibility"]
---

## Goal
Owner request: fix the review nits left open from M0/M1, adding no features. Each fix comes with a test. It has no deps and runs in parallel with every server, parse, CLI and harness task in M2. It overlaps 034/035/036 on `Shell.svelte`/`Detail.svelte` (a normal edit against their `+` entries), so they list it in their deps.

## Acceptance criteria
- [ ] 1. After a successful login, the Shell's `command` input has focus (`document.activeElement`), not `body`. Tested in `App.test.ts`.
- [ ] 2. After setup gets `409 owner already exists`, the setup screen shows `owner already exists` and a `[ log in ]` action that opens the login screen. Tested in `App.test.ts` (409 → click → login screen).
- [ ] 3. The logout test in `App.test.ts` reaches the `logout` button with `userEvent.tab()` and asserts it is focused. It no longer calls `.focus()` in code.
- [ ] 4. `Esc` on the detail page does nothing while focus is in the Shell `command` input or any control that owns keys (`ownsKeys`). Test: typing in the command box and pressing `Esc` keeps `#/o/<id>`, while `Esc` on the page body still goes back.
- [ ] 5. `j`/`k`/`Enter` list navigation works while focus is on a tag-folder button in `TagTree`. Other buttons and inputs still swallow the keys. Tested in `List.test.ts`.
- [ ] 6. Focus is restored after each of these:
  - link (to `[ link ]`)
  - unlink (to the next row's `[ unlink ]`, else `[ link ]`)
  - comment edit save (to that comment's `[ edit ]`)
  - comment delete (to the next comment's `[ edit ]`, else the `add comment` textarea)
  - One test per case in `Links.test.ts`/`Comments.test.ts`, asserting `document.activeElement`.
- [ ] 7. In `Comments`, `<created>`, `[ edit ]` and `[ delete ]` sit in one header row. Both buttons stay in one non-wrapping group, and the timestamp may wrap above them. `Comments.test.ts` asserts both buttons share one group element.
  - This task doesn't touch `web/e2e/` (that would block running it in parallel with 033). The 360 px browser check, that both buttons have the same `top`, is listed as a follow-up in the PR.
- [ ] 8. `polykov edit <id>` with no change flags exits 2 with a one-line usage error on stderr naming the change flags, and sends no request. The test in `cli/tests/object.rs` checks the exit code and that the object's `updated` is unchanged.
- [ ] 9. `Client::put` has a unit test in `cli/src/client.rs`, against a local test server, covering a `204` reply and a JSON error reply.
- [ ] `scripts/verify.sh` passes, and `web/API.md`/`cli/API.md` are updated wherever a behaviour above is documented.

## Out of scope
- Any new feature or layout change beyond items 1–9. The other M1 phone-check failures found by 033 are separate follow-ups.

## Notes for next
- Added dev dep `@testing-library/user-event` (web/package.json + lock, outside `touches`) for criterion 3's `userEvent.tab()`.
- Shell takes `focusCommand`; App sets it only after a fresh login, so a reload with a session keeps focus on body (j/k on the list still work).
- After a setup 409 the same AuthPanel turns into `[ log in ]` with the fields hidden (no AuthPanel change needed).
- Detail's Esc now ignores any `ownsKeys` target, buttons included; List lets `[data-tag-folder]` buttons through.
- Follow-up (e2e, not touched to stay parallel with 033): at 360 px, check `[ edit ]`/`[ delete ]` share the same `top` in Comments.
