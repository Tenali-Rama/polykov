# 0016 — Saved views

- **Status**: accepted
- **Context**: Spec "Organization" and "Tasks (GTD)" ask for saved views with their own layout, grouping and sort, pinnable to the sidebar (task 043). Every client must render them the same way.
- **Decision**: A view is an object of type `view`, no new table. Its `properties` hold `query` (ADR 0015 grammar), `layout` (`list|kanban|eisenhower`), `group` (`none|status|priority|project|section|area|tag|type`) and `sort` (`smart|priority|deadline|scheduled|created|updated|completed|title`); pinning is the object `pinned` flag; rename, delete and pin use the object routes. The web route for a view is `#/v/<id>`.
  - Sort directions are fixed: `created`/`updated`/`completed` newest first (ties by id descending), the rest ascending (ties by id ascending), `smart` by ADR 0017. Unset values go last.
  - `GET /api/views/{id}/items` sorts every match, keeps the first 500, then groups them. Group keys with no value (`null`, label `none`) go last; `tag` groups by an object's first tag; `area` is a task's own area, else its project's.
  - `kanban` with `group: none` groups by `status`. `eisenhower` ignores `group`: four fixed quadrants; important is priority ≤ 2; urgent is a deadline or scheduled date on or before today + 2 days (overdue included).
- **Consequences**: Clients only render the groups the server returns. 500, the 2-day horizon and first-tag grouping are defaults the owner may change in `server/src/views/`. Calendar and timeline layouts (M8) add layout values; existing values must keep their meaning.
