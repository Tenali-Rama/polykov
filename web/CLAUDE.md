# web/

Svelte 5 + TypeScript single-page app built with Vite (no SvelteKit). The server serves `dist/` (task 007); Tauri wraps it later.

## Commands (run from repo root)
- `npm --prefix web ci` — install exact deps from `package-lock.json`.
- `npm --prefix web run dev` — dev server; proxies `/health` and `/api` to `http://127.0.0.1:7700`.
- `npm --prefix web run lint | check | test | build` — the steps `scripts/verify.sh` runs.

## Layout
- `src/main.ts` mounts `App.svelte`; `src/app.css` holds global tokens (palette, font, focus style).
- `src/lib/` holds non-UI modules (API client). See `API.md`.
- Tests sit next to the code as `*.test.ts` (Vitest + jsdom + Testing Library).

## Rules
- Terminal look (spec §7): monospace, box borders, prompt-style inputs, block cursor, restrained palette, no icon library.
- Colors only via the CSS variables in `app.css`; light and dark come from `prefers-color-scheme`.
- Keyboard-first: every control reachable by Tab with a visible focus style; touch targets ≥ 44px; no horizontal scroll at 360px.
- All server calls go through `src/lib/api.ts`; same-origin relative paths only.
- Mock `fetch` with `vi.stubGlobal` in tests; never hit a real server.
