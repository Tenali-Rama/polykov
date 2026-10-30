# 0017 — Smart sort

- **Status**: accepted
- **Context**: Spec "Tasks (GTD)" asks for a smart sort by priority, deadline proximity, estimate and goal relevance (task 042). Lists now and saved views (043) need one shared, explainable order.
- **Decision**: `server/src/smartsort/` scores a task as of `today` (higher first, ties by ascending `id`): priority `10 × (4 − p)` (P1 30 … P4 0); deadline overdue `40` flat, else `30 × max(0, 1 − days/30)` (today 30, 30+ days 0, none 0); estimate `5 × max(0, 1 − minutes/120)` (none 0); goal relevance `20 × r` with `r` fixed at 0 until goals land in M9 (owner may change). A date-time deadline counts by its date. The weights are the constants in `smartsort/mod.rs`, nowhere else.
- **Consequences**: An overdue P3 (50) ranks above a P1 due in 30 days (30), and an overdue task outranks any undated one; the quick-win bonus (≤ 5) never outweighs a priority level. `sort=smart` scores every match of a list, then pages, so it costs a full scan of the list. Change weights only here and in this ADR; M12 learned ranking replaces the score, not the API.
