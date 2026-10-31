# 0022 — Layouts for GTD lists

- **Status**: accepted
- **Context**: Spec "Tasks (GTD)" makes layouts available for any task list or view. ADR 0016 gave them only to saved views; the GTD lists and the project view (`all?project=`) need the same Kanban and Eisenhower shapes (task 060).
- **Decision**: `GET /api/lists/{name}/items?layout=&group=` selects and orders tasks exactly as `GET /api/lists/{name}`, keeps the first 500, reads their view keys (`views::keys::of`) and groups them with `views::items::group`. There is one grouping and one quadrant rule, shared with views, so a list and a view with the same tasks and settings render the same groups.
  - Layout and group are request parameters, not stored: which layout a list shows is a client setting (the web keeps it locally, task 061). Lists have no stored settings object.
  - Sort stays the list's own (`default|smart`); the view sorts don't apply to lists.
- **Consequences**: A change to the view grouping (new group kinds, the urgent horizon) changes lists too; keep them shared. Calendar and timeline layouts (M8) add layout values to both at once.
