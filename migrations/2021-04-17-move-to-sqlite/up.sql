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
  attribution_id INT,
  width INT NOT NULL,
  height INT NOT NULL,
  thumbnail BLOB NOT NULL
);

CREATE INDEX photos_date_idx ON photos (date DESC);
CREATE INDEX photos_grade_idx ON photos (grade DESC);

--
-- Tags
--
CREATE TABLE tags (
  id INTEGER NOT NULL PRIMARY KEY,
  slug TEXT UNIQUE NOT NULL,
  tag_name TEXT UNIQUE NOT NULL
);

CREATE TABLE photo_tags (
  id INTEGER NOT NULL PRIMARY KEY,
  photo_id TEXT NOT NULL NOT NULL REFERENCES photos (id),
  tag_id INTEGER NOT NULL NOT NULL REFERENCES tags (id)
);

--
-- Attributions
--
CREATE TABLE attributions (
  id INTEGER NOT NULL PRIMARY KEY,
  name TEXT UNIQUE NOT NULL
);
