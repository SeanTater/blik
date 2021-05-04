--
-- Photos
--
CREATE TABLE photos (
  id TEXT NOT NULL PRIMARY KEY,
  path TEXT UNIQUE NOT NULL,
  date TIMESTAMP,
  year INT NOT NULL,
  month INT NOT NULL,
  day INT NOT NULL,
  grade SMALLINT,
  rotation SMALLINT NOT NULL,
  is_public BOOLEAN NOT NULL DEFAULT 0,
  width INT NOT NULL,
  height INT NOT NULL,
  story TEXT NOT NULL,
  lat FLOAT,
  lon FLOAT
);

CREATE INDEX photos_date_idx ON photos (date DESC);
CREATE INDEX photos_grade_idx ON photos (grade DESC);
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
  description TEXT,
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
  -- Any detauls about this annotation, preferably JSON
  -- so it can be human readable too
  details TEXT,
  -- You can't duplicate all three columns
  PRIMARY KEY (photo_id, name, details)
) WITHOUT ROWID;
