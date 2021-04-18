--
-- Photos
--
CREATE TABLE photos (
  id INTEGER NOT NULL PRIMARY KEY,
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
  height INT NOT NULL
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
  photo_id INTEGER NOT NULL NOT NULL REFERENCES photos (id),
  tag_id INTEGER NOT NULL NOT NULL REFERENCES tags (id)
);

--
-- People
--
CREATE TABLE people (
  id INTEGER NOT NULL PRIMARY KEY,
  slug TEXT UNIQUE NOT NULL,
  person_name TEXT UNIQUE NOT NULL
);

CREATE TABLE photo_people (
  id INTEGER NOT NULL PRIMARY KEY,
  photo_id INTEGER NOT NULL NOT NULL REFERENCES photos (id),
  person_id INTEGER NOT NULL NOT NULL REFERENCES people (id)
);

--
-- Places
-- 
CREATE TABLE places (
  id INTEGER NOT NULL PRIMARY KEY,
  slug TEXT UNIQUE NOT NULL,
  place_name TEXT UNIQUE NOT NULL,
  osm_id BIGINT UNIQUE,
  osm_level SMALLINT
);

CREATE TABLE photo_places (
  id INTEGER NOT NULL PRIMARY KEY,
  photo_id INTEGER NOT NULL NOT NULL REFERENCES photos (id),
  place_id INTEGER NOT NULL NOT NULL REFERENCES places (id)
);

CREATE INDEX places_osml_idx ON places (osm_level);
CREATE UNIQUE INDEX places_name_idx ON places (place_name, osm_level);

--
-- Positions
--
-- Rather than using floating points or DECIMAL(8,5) or something like
-- that, lat and long are stored as signed microdegrees integer values.
CREATE TABLE positions (
  id INTEGER NOT NULL PRIMARY KEY,
  photo_id INTEGER NOT NULL UNIQUE NOT NULL REFERENCES photos (id),
  latitude INTEGER NOT NULL,
  longitude INTEGER NOT NULL
);

CREATE INDEX positions_photo_idx ON positions (photo_id);
CREATE INDEX positions_lat_idx ON positions (latitude);
CREATE INDEX positions_long_idx ON positions (longitude);

--
-- Attributions
--
CREATE TABLE attributions (
  id INTEGER NOT NULL PRIMARY KEY,
  name TEXT UNIQUE NOT NULL
);
