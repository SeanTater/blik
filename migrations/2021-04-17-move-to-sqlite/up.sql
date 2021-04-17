--
-- Photos
--
CREATE TABLE photos (
  id INTEGER PRIMARY KEY,
  path TEXT UNIQUE NOT NULL,
  date TEXT,
  grade INT,
  rotation INT NOT NULL,
  is_public INT DEFAULT 0,
  camera_id INT,
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
  id INTEGER PRIMARY KEY,
  slug TEXT UNIQUE NOT NULL,
  tag_name TEXT UNIQUE NOT NULL
);

CREATE TABLE photo_tags (
  id INTEGER PRIMARY KEY,
  photo_id INTEGER NOT NULL REFERENCES photos (id),
  tag_id INTEGER NOT NULL REFERENCES tags (id)
);

--
-- People
--
CREATE TABLE people (
  id INTEGER PRIMARY KEY,
  slug TEXT UNIQUE NOT NULL,
  person_name TEXT UNIQUE NOT NULL
);

CREATE TABLE photo_people (
  id INTEGER PRIMARY KEY,
  photo_id INTEGER NOT NULL REFERENCES photos (id),
  person_id INTEGER NOT NULL REFERENCES people (id)
);

--
-- Places
-- 
CREATE TABLE places (
  id INTEGER PRIMARY KEY,
  slug TEXT UNIQUE NOT NULL,
  place_name TEXT UNIQUE NOT NULL,
  osm_id INT UNIQUE,
  osm_level INT
);

CREATE TABLE photo_places (
  id INTEGER PRIMARY KEY,
  photo_id INTEGER NOT NULL REFERENCES photos (id),
  place_id INTEGER NOT NULL REFERENCES places (id)
);

CREATE INDEX places_osml_idx ON places (osm_level);
CREATE UNIQUE INDEX places_name_idx ON places (place_name, osm_level);


--
-- Users
--
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  username TEXT UNIQUE NOT NULL,
  password TEXT UNIQUE NOT NULL
);

--
-- Positions
--
-- Rather than using floating points or DECIMAL(8,5) or something like
-- that, lat and long are stored as signed microdegrees integer values.
CREATE TABLE positions (
  id INTEGER PRIMARY KEY,
  photo_id INTEGER UNIQUE NOT NULL REFERENCES photos (id),
  latitude INTEGER NOT NULL,
  longitude INTEGER NOT NULL
);

CREATE INDEX positions_photo_idx ON positions (photo_id);
CREATE INDEX positions_lat_idx ON positions (latitude);
CREATE INDEX positions_long_idx ON positions (longitude);


--
-- Cameras
--
CREATE TABLE cameras (
  id           INTEGER PRIMARY KEY,
  manufacturer TEXT NOT NULL,
  model        TEXT NOT NULL
);

CREATE UNIQUE INDEX cameras_idx ON cameras (manufacturer, model);


--
-- Attributions
--
CREATE TABLE attributions (
  id INTEGER PRIMARY KEY,
  name TEXT UNIQUE NOT NULL
);
