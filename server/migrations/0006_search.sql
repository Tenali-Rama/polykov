-- The global search index (spec "Organization", ADR 0014): one FTS5 row per live object,
-- keyed by the object id, holding its title, tags, the string values of its properties and
-- its comment bodies. The trigram tokenizer makes every substring of 3+ characters
-- matchable; fuzzy matching and ranking happen in `search::store`. Trashed objects have no
-- row, so they never match; restoring one re-indexes it.
CREATE VIRTUAL TABLE search USING fts5 (
    title, tags, props, comments,
    tokenize = 'trigram'
);

-- What the index holds for each live object.
CREATE VIEW search_source AS
SELECT o.id AS id,
       o.title AS title,
       COALESCE((SELECT group_concat(tag, ' ') FROM object_tags WHERE object_id = o.id), '')
           AS tags,
       COALESCE((SELECT group_concat(value, ' ') FROM json_tree(o.properties)
                 WHERE type = 'text'), '') AS props,
       COALESCE((SELECT group_concat(body, ' ') FROM comments WHERE object_id = o.id), '')
           AS comments
FROM objects o
WHERE o.deleted_at IS NULL;

INSERT INTO search (rowid, title, tags, props, comments)
SELECT id, title, tags, props, comments FROM search_source;

-- Every trigger below rebuilds the row of the object it touched; the view yields nothing
-- for a trashed or deleted object, so its row just goes away.
CREATE TRIGGER search_object_insert AFTER INSERT ON objects BEGIN
    INSERT INTO search (rowid, title, tags, props, comments)
    SELECT id, title, tags, props, comments FROM search_source WHERE id = new.id;
END;

CREATE TRIGGER search_object_update AFTER UPDATE OF title, properties, deleted_at ON objects
BEGIN
    DELETE FROM search WHERE rowid = old.id;
    INSERT INTO search (rowid, title, tags, props, comments)
    SELECT id, title, tags, props, comments FROM search_source WHERE id = new.id;
END;

CREATE TRIGGER search_object_delete AFTER DELETE ON objects BEGIN
    DELETE FROM search WHERE rowid = old.id;
END;

CREATE TRIGGER search_tag_insert AFTER INSERT ON object_tags BEGIN
    DELETE FROM search WHERE rowid = new.object_id;
    INSERT INTO search (rowid, title, tags, props, comments)
    SELECT id, title, tags, props, comments FROM search_source WHERE id = new.object_id;
END;

CREATE TRIGGER search_tag_delete AFTER DELETE ON object_tags BEGIN
    DELETE FROM search WHERE rowid = old.object_id;
    INSERT INTO search (rowid, title, tags, props, comments)
    SELECT id, title, tags, props, comments FROM search_source WHERE id = old.object_id;
END;

CREATE TRIGGER search_comment_insert AFTER INSERT ON comments BEGIN
    DELETE FROM search WHERE rowid = new.object_id;
    INSERT INTO search (rowid, title, tags, props, comments)
    SELECT id, title, tags, props, comments FROM search_source WHERE id = new.object_id;
END;

CREATE TRIGGER search_comment_update AFTER UPDATE OF body ON comments BEGIN
    DELETE FROM search WHERE rowid = new.object_id;
    INSERT INTO search (rowid, title, tags, props, comments)
    SELECT id, title, tags, props, comments FROM search_source WHERE id = new.object_id;
END;

CREATE TRIGGER search_comment_delete AFTER DELETE ON comments BEGIN
    DELETE FROM search WHERE rowid = old.object_id;
    INSERT INTO search (rowid, title, tags, props, comments)
    SELECT id, title, tags, props, comments FROM search_source WHERE id = old.object_id;
END;
