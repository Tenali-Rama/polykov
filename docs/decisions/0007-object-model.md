# 0007 — Object model

- **Status**: accepted
- **Context**: Spec §3 has one base Object for every type, with tags, links, comments and attachments. M1 (tasks 013–017) builds all of it, and later tasks should not need new migrations for it.
- **Decision**: One `objects` table holds every type (`type` column, `properties` JSON text for custom fields). Tags, links, comments and attachments are side tables with FKs `ON DELETE CASCADE` to `objects`. Ids are `INTEGER PRIMARY KEY AUTOINCREMENT`, so an id is never handed out again, even after a purge. All times are Unix seconds, served as RFC 3339 UTC. Delete is soft (`deleted_at`). Relations are served as sub-resources (e.g. `/api/objects/{id}/links`), not embedded in the object JSON; only `tags` is embedded.
- **Consequences**: A new type is a list entry plus validation, not a table. Type-specific fields later go in `properties` or new columns on `objects`. Every live read must filter `deleted_at IS NULL` (`objects::store` does, and `summary` is how other modules check a target). Permanent delete is a plain `DELETE FROM objects`, which cascades. Don't drop AUTOINCREMENT: ids may be referenced from outside (links in notes, CLI scripts).
