---
id: 038
title: Fix the M1 phone-fit failures and check comment actions at 360 px
milestone: M2
status: done
deps: [037]
touches: [web/src/objects/List.svelte, web/src/objects/TagsEditor.svelte, web/src/objects/Comments.svelte, web/e2e/m1.spec.ts, web/API.md]
spec: ["7. Clients & UI", "8. Customization & Accessibility"]
---

## Goal
Follow-up to 033 and 037. The three phone-fit failures that 033's harness found, now marked `test.fixme` in `web/e2e/m1.spec.ts`, are fixed and their tests pass. The 360 px browser check for comment actions that 037 deferred also exists and passes. No new features.

Deps: 037 edits `List.svelte` and `Comments.svelte` and adds the comment header group that the check below measures. 034–036 touch none of these files (`Detail.svelte`/`Shell.svelte` aren't edited here), so this task runs in parallel with them.

## Acceptance criteria
- [ ] 1. List: the `pinned only` checkbox (`List.svelte`) has a hit box of at least 44×44 px at 360 px. It's still a native `<input type="checkbox">` with the label `pinned only`, and toggling it still filters.
- [ ] 2. List: the `trash` link (`.trashlink`) is at least 44 px wide and 44 px tall at 360 px. Its text stays `trash` (padding or `min-width`, not new text).
- [ ] 3. Detail: each tag's `[ x ]` remove button (`TagsEditor.svelte`, `aria-label="remove tag <tag>"`) is at least 44×44 px at 360 px.
- [ ] 4. Both `test.fixme(...)` lines in the `phone fit` block of `web/e2e/m1.spec.ts` (`object list`, `detail with links, comments and attachments`) are removed. `grep -n fixme web/e2e/m1.spec.ts` prints nothing, and both tests pass in the `phone` project.
- [ ] 5. New test in the `phone fit` block of `web/e2e/m1.spec.ts`, `comment actions`: at 360 px, on an object with a comment, that comment's `[ edit ]` and `[ delete ]` buttons have the same `boundingBox().y` (±1 px), and each is at least 44×44 px. If it fails, fix the layout in `Comments.svelte`.
- [ ] 6. Fixes stay inside the component styles. The 360 px fit of the other views does not regress: every `phone fit` test passes.
- [ ] 7. `scripts/e2e.sh` and `scripts/verify.sh` pass. If `web/API.md` mentions the known phone-fit gaps, remove that mention.

## Out of scope
- Global style changes in `app.css` or `Shell.svelte`, and any layout change beyond the elements above.
- Phone-fit checks for the M2 task views. 034–036 own those in their own e2e files.

## Notes for next
- The `pinned only` checkbox is sized to 44×44 px itself (`width`/`height`), because `expectPhoneFit` measures the `<input>`, not its `<label>`.
- `.trashlink` and the tag `[ x ]` button got `min-width: 44px`; their text is unchanged.
- `Comments.svelte` needed no change: `.ops` already keeps `[ edit ]` and `[ delete ]` on one line at 360 px.
- `web/API.md` didn't mention the phone-fit gaps, so it's unchanged.
- `web/CLAUDE.md`'s example `scripts/e2e.sh --project phone m1` fails (Playwright reads `m1` as a second project); use `--project=phone m1`. Not fixed here: outside touches.
