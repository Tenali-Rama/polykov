# 0010 — Browser e2e tests

- **Status**: accepted
- **Context**: Spec §10 asks for e2e UI tests. In M1, layout bugs (overflow at 360 px, targets under 44 px) passed jsdom tests and only showed up in a real browser.
- **Decision**:
  - Playwright (`@playwright/test`) in `web/e2e/`, Chromium only, two projects: `phone` (360×740, touch) and `desktop` (1280×800).
  - `scripts/e2e.sh` starts a real `polykov-server` per run (free port, temp data dir, `web/dist`), sets the owner up, and runs every test against it. It is the verify step `e2e`.
  - Tests create their own data over REST and never depend on each other, so they run fully parallel on one server.
- **Consequences**: No mocked API in e2e, so a server change that breaks the UI fails verify. Other browsers and Tauri device tests are out of scope until M13/M14. Don't share data between tests or reuse a server across runs.
