-- The M1 object model (spec §3, ADR 0007): one `objects` table for every type, plus side
-- tables for tags, links, comments and attachments. Times are Unix seconds. Ids are
-- AUTOINCREMENT so a deleted id is never handed out again.
CREATE TABLE objects (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    type       TEXT NOT NULL,
    title      TEXT NOT NULL,
    properties TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties)),
    pinned     INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    archived   INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    created    INTEGER NOT NULL DEFAULT (unixepoch()),
    updated    INTEGER NOT NULL DEFAULT (unixepoch()),
    -- Set when moved to the trash; NULL for live objects.
    deleted_at INTEGER
) STRICT;

CREATE TABLE object_tags (
    object_id INTEGER NOT NULL REFERENCES objects (id) ON DELETE CASCADE,
    tag       TEXT NOT NULL,
    PRIMARY KEY (object_id, tag)
) STRICT;
CREATE INDEX object_tags_tag ON object_tags (tag);

CREATE TABLE links (
    src_id  INTEGER NOT NULL REFERENCES objects (id) ON DELETE CASCADE,
    dst_id  INTEGER NOT NULL REFERENCES objects (id) ON DELETE CASCADE,
    created INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (src_id, dst_id),
    CHECK (src_id <> dst_id)
) STRICT;
CREATE INDEX links_dst ON links (dst_id);

CREATE TABLE comments (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    object_id INTEGER NOT NULL REFERENCES objects (id) ON DELETE CASCADE,
    body      TEXT NOT NULL,
    created   INTEGER NOT NULL DEFAULT (unixepoch()),
    updated   INTEGER NOT NULL DEFAULT (unixepoch())
) STRICT;
CREATE INDEX comments_object ON comments (object_id);

CREATE TABLE attachments (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    object_id INTEGER NOT NULL REFERENCES objects (id) ON DELETE CASCADE,
    name      TEXT NOT NULL,
    mime      TEXT NOT NULL,
    size      INTEGER NOT NULL,
    created   INTEGER NOT NULL DEFAULT (unixepoch())
) STRICT;
CREATE INDEX attachments_object ON attachments (object_id);
