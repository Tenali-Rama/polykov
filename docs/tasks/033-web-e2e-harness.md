---
id: 033
title: Add a headless-browser e2e harness with 360 px phone checks
milestone: M2
status: done
deps: []
touches: [web/package.json, web/package-lock.json, web/playwright.config.ts, web/e2e/, web/eslint.config.js, web/tsconfig.json, web/CLAUDE.md, web/API.md, scripts/e2e.sh, scripts/verify.sh, .github/workflows/ci.yml, .gitignore, docs/decisions/]
spec: ["10. Non-Functional", "7. Clients & UI", "8. Customization & Accessibility"]
---

## Goal
Core UI flows run in a real headless Chromium against the real server, as spec §10 asks for e2e UI tests. Every UI task can then prove three things at 360 px: no horizontal scroll, 44 px touch targets, and a working keyboard flow. In M1, bugs like these only showed up in a real browser.

## Acceptance criteria
- [ ] `@playwright/test` is a dev dependency in `web/`. `web/playwright.config.ts` defines two projects: `phone` (360×740, touch) and `desktop` (1280×800). Chromium only.
- [ ] `scripts/e2e.sh` does the following:
  - builds the server if needed and starts it on a free port, with a temp data dir and `web/dist`
  - runs `POST /api/setup`
  - runs `npx playwright test` with `BASE_URL`
  - stops the server and removes the temp dir on exit
  - prints `e2e: ok` or the first failure
  - installs Chromium (`playwright install chromium`) if missing (default — owner may change)
- [ ] `scripts/verify.sh` runs it as step `e2e` after `smoke`. `.github/workflows/ci.yml` installs Chromium with system deps before verify.
- [ ] Helpers in `web/e2e/`:
  - `login(page)`, plus a global setup that saves the logged-in storage state
  - `api(request)` to create test data over REST. Each test makes its own data, so tests are independent.
  - `expectPhoneFit(page)`: fails, naming the element, if `document.documentElement.scrollWidth > innerWidth`, or if any visible `a, button, input, select, textarea, [role=button], [tabindex]` has a box under 44×44 px
  - `tabTo(page, name)`: presses Tab until the focused element has that accessible name, failing after 50 presses
- [ ] `web/e2e/m1.spec.ts` covers login, the object list, detail with links, comments and attachments, and the trash. Each screen gets `expectPhoneFit` on `phone`, plus one keyboard flow: log in with only the keyboard, `j`/`k`/`Enter` to open an object, `Esc` back.
  - Any check that fails on today's M1 UI is marked `test.fixme('<one-line cause>')` and listed in the PR as a follow-up. This task doesn't edit `web/src/`.
- [ ] `web/CLAUDE.md` gains a rule: every UI task adds `web/e2e/<feature>.spec.ts` with `expectPhoneFit` and a keyboard flow. Lint and type-check cover `web/e2e/`.
- [ ] `scripts/verify.sh` passes locally with step `PASS e2e`.
- [ ] A new ADR, with the next free number, `docs/decisions/NNNN-browser-e2e.md`, covers Playwright, Chromium only, and a real server per run.

## Out of scope
- Fixing M1 layout bugs. Task 037 and follow-ups do that.
- Tauri/Android device tests (M13/M14).

## Notes for next
- Run one spec: `scripts/e2e.sh --project phone m1` (extra args go to `playwright test`). Logins wait 30 s: argon2 in the debug server takes seconds under parallel load.
- `expectPhoneFit` compares against the configured viewport width, not `innerWidth`: mobile Chromium widens `innerWidth` to fit overflowing content, which hides the scroll.
- Phone-fit fixmes on today's UI (follow-ups): list `pinned only` checkbox 13×13 and `trash` link 38 px wide; detail tag `[ x ]` 38 px wide. Login and trash fit.
- After 037 focuses the command input on login, the keyboard flow still works: it Tabs to the first row before `j`/`k`.
- `vitest run --dir src` keeps Vitest away from `e2e/*.spec.ts`; tsconfig types now include `node` for the e2e files.
