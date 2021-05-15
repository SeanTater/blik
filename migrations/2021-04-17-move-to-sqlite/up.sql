--
-- Photos
--
CREATE TABLE photos (
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
);

CREATE INDEX photos_date_idx ON photos (date DESC);
CREATE INDEX photos_story_idx ON photos (story);

--
-- Thumbnails, separated from photos both to make debugging easier
-- and to improve performance for queries that don't use it.
--
CREATE TABLE thumbnail (
  id TEXT NOT NULL PRIMARY KEY,
  content BLOB NOT NULL
);

--
-- Story
--
CREATE TABLE story (
  name TEXT NOT NULL PRIMARY KEY,
  description TEXT NOT NULL,
  created_on TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

--
-- Annotation
--
CREATE TABLE annotation (
  -- Attached to a specific photo
  photo_id TEXT NOT NULL REFERENCES photos (id),
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
  PRIMARY KEY (photo_id, name, top, bottom, left, right, details)
) WITHOUT ROWID;
