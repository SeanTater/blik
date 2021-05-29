--
-- Photos
--
CREATE TABLE media (
    id TEXT NOT NULL PRIMARY KEY,
    path TEXT UNIQUE NOT NULL,
    date TIMESTAMP,
    rotation SMALLINT NOT NULL,
    is_public BOOLEAN NOT NULL DEFAULT 0,
    width INT NOT NULL,
    height INT NOT NULL,
    story TEXT NOT NULL,
    lat DOUBLE,
    lon DOUBLE,
    make TEXT,
    model TEXT,
    caption TEXT
) WITHOUT ROWID;

CREATE INDEX media_date_idx ON media (date DESC);
CREATE INDEX media_story_idx ON media (story);

--
-- Thumbnails, separated from photos both to make debugging easier
-- and to improve performance for queries that don't use it.
--
CREATE TABLE thumbnail (
    id TEXT NOT NULL PRIMARY KEY,
    content BLOB NOT NULL
) WITHOUT ROWID;

--
-- Story
--
CREATE TABLE story (
    name TEXT NOT NULL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_updated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    latest_media TEXT REFERENCES media(id),
    media_count INTEGER NOT NULL DEFAULT 0
);

CREATE TRIGGER update_story_on_upload
AFTER INSERT ON media FOR EACH ROW
BEGIN
    UPDATE story
    SET last_updated = CURRENT_TIMESTAMP,
        latest_media = NEW.id,
        media_count = media_count + 1
    WHERE story.name = NEW.story;
END;

CREATE TRIGGER update_story_on_delete
AFTER DELETE ON media FOR EACH ROW
BEGIN
    UPDATE story
    SET last_updated = CURRENT_TIMESTAMP,
        latest_media = (
            SELECT id FROM media
            WHERE media.story = OLD.story
            ORDER BY date DESC
            LIMIT 1
        ),
        media_count = media_count - 1
    WHERE story.name = OLD.story;
END;

--
-- Annotation
--
CREATE TABLE annotation (
    -- Attached to a specific photo
    media_id TEXT NOT NULL REFERENCES media (id),
    -- A type of tag, like "caption"
    name TEXT NOT NULL,
    -- A region the tag applies to
    top INTEGER NOT NULL DEFAULT 0,
    bottom INTEGER NOT NULL DEFAULT 0,
    left INTEGER NOT NULL DEFAULT 0,
    right INTEGER NOT NULL DEFAULT 0,
    -- Any details about this annotation, preferably JSON
    -- so it can be human readable too
    details TEXT,
    -- You can't duplicate all three columns
    PRIMARY KEY (media_id, name, top, bottom, left, right, details)
) WITHOUT ROWID;
