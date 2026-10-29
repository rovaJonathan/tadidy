-- The whole data model of the spec (04-architecture.md#data-model) in one step, so phase 2
-- (pinned clips, snippets, collections, images, excluded apps) adds no table rewrite.

CREATE TABLE collections (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sort REAL NOT NULL
);

CREATE TABLE clips (
    id             TEXT PRIMARY KEY,                                        -- UUID v7
    kind           TEXT NOT NULL,                                           -- 'text'|'rich'|'image'|'file'|'url'|'color'
    content_hash   TEXT NOT NULL UNIQUE,                                    -- SHA-256 of the primary representation
    plain_text     TEXT,                                                    -- searchable text
    preview        TEXT,                                                    -- truncated preview (300 chars)
    title          TEXT,                                                    -- snippets
    source_bundle  TEXT,
    source_name    TEXT,
    byte_size      INTEGER NOT NULL,
    width          INTEGER,
    height         INTEGER,
    pinned         INTEGER NOT NULL DEFAULT 0,
    pin_order      REAL,
    collection_id  TEXT REFERENCES collections(id) ON DELETE SET NULL,
    copy_count     INTEGER NOT NULL DEFAULT 1,
    created_at     INTEGER NOT NULL,                                        -- ms epoch, first copy
    last_copied_at INTEGER NOT NULL,                                        -- main sort key
    last_used_at   INTEGER                                                  -- last paste from Tadidy
);
CREATE INDEX idx_clips_recent ON clips(pinned, last_copied_at DESC);
CREATE INDEX idx_clips_kind ON clips(kind, last_copied_at DESC);
-- The unfiltered newest-first list; `id` makes the order total, so a cursor pages without gaps.
CREATE INDEX idx_clips_last_copied ON clips(last_copied_at DESC, id DESC);

CREATE TABLE representations (
    clip_id TEXT NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
    uti     TEXT NOT NULL,                                                  -- public.rtf, public.png, public.file-url…
    data    BLOB NOT NULL,
    PRIMARY KEY (clip_id, uti)
);

CREATE TABLE thumbnails (
    clip_id TEXT PRIMARY KEY REFERENCES clips(id) ON DELETE CASCADE,
    png     BLOB NOT NULL
);

CREATE TABLE excluded_apps (
    bundle_id TEXT PRIMARY KEY,
    name      TEXT NOT NULL
);

CREATE TABLE app_icons (                                                    -- source app icon cache
    bundle_id TEXT PRIMARY KEY,
    png       BLOB NOT NULL
);

-- External-content index over `clips`, keyed by its implicit rowid. A full VACUUM may renumber
-- rowids: only `PRAGMA incremental_vacuum` is used, and any future VACUUM must be followed by
-- INSERT INTO clips_fts(clips_fts) VALUES ('rebuild').
CREATE VIRTUAL TABLE clips_fts USING fts5(
    plain_text, title, source_name,
    content = 'clips', content_rowid = 'rowid',
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER clips_ai AFTER INSERT ON clips BEGIN
    INSERT INTO clips_fts(rowid, plain_text, title, source_name)
    VALUES (new.rowid, new.plain_text, new.title, new.source_name);
END;

CREATE TRIGGER clips_ad AFTER DELETE ON clips BEGIN
    INSERT INTO clips_fts(clips_fts, rowid, plain_text, title, source_name)
    VALUES ('delete', old.rowid, old.plain_text, old.title, old.source_name);
END;

-- Fires only for the indexed columns, and only when one of them changed: a bump that lists
-- `source_name` in its SET clause with the same value does not rewrite the index.
CREATE TRIGGER clips_au AFTER UPDATE OF plain_text, title, source_name ON clips
WHEN old.plain_text IS NOT new.plain_text
  OR old.title IS NOT new.title
  OR old.source_name IS NOT new.source_name
BEGIN
    INSERT INTO clips_fts(clips_fts, rowid, plain_text, title, source_name)
    VALUES ('delete', old.rowid, old.plain_text, old.title, old.source_name);
    INSERT INTO clips_fts(rowid, plain_text, title, source_name)
    VALUES (new.rowid, new.plain_text, new.title, new.source_name);
END;
