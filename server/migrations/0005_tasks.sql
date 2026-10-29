-- The M2 task model (spec §3 "Task" and "Project", ADR 0011): type-specific fields live in
-- side tables keyed by the object id. Times are Unix seconds; `scheduled`, `deadline` and
-- `start` are floating local dates as TEXT (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`).
CREATE TABLE sections (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES objects (id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    position   INTEGER NOT NULL DEFAULT 0,
    created    INTEGER NOT NULL DEFAULT (unixepoch())
) STRICT;
CREATE INDEX sections_project ON sections (project_id);

CREATE TABLE projects (
    object_id INTEGER PRIMARY KEY REFERENCES objects (id) ON DELETE CASCADE,
    status    TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'someday', 'done')),
    area_id   INTEGER REFERENCES objects (id) ON DELETE SET NULL,
    start     TEXT,
    deadline  TEXT
) STRICT;

CREATE TABLE tasks (
    object_id  INTEGER PRIMARY KEY REFERENCES objects (id) ON DELETE CASCADE,
    status     TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'done')),
    -- 1 is the highest; 4 means "no priority".
    priority   INTEGER NOT NULL DEFAULT 4 CHECK (priority BETWEEN 1 AND 4),
    scheduled  TEXT,
    deadline   TEXT,
    -- Whole minutes.
    estimate   INTEGER CHECK (estimate BETWEEN 1 AND 10080),
    someday    INTEGER NOT NULL DEFAULT 0 CHECK (someday IN (0, 1)),
    parent_id  INTEGER REFERENCES objects (id) ON DELETE SET NULL,
    project_id INTEGER REFERENCES objects (id) ON DELETE SET NULL,
    section_id INTEGER REFERENCES sections (id) ON DELETE SET NULL,
    area_id    INTEGER REFERENCES objects (id) ON DELETE SET NULL,
    -- JSON array of {"text", "done"}.
    checklist  TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(checklist)),
    -- JSON rule; NULL until recurrence (task 027).
    recurrence TEXT CHECK (recurrence IS NULL OR json_valid(recurrence)),
    -- Set when status becomes 'done'.
    completed  INTEGER,
    CHECK (parent_id IS NULL OR parent_id <> object_id)
) STRICT;
CREATE INDEX tasks_parent ON tasks (parent_id);
CREATE INDEX tasks_project ON tasks (project_id);
CREATE INDEX tasks_area ON tasks (area_id);
CREATE INDEX tasks_scheduled ON tasks (scheduled);
CREATE INDEX tasks_deadline ON tasks (deadline);
