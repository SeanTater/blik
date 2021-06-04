ALTER TABLE media ADD COLUMN mimetype TEXT NOT NULL DEFAULT 'image/jpeg';
ALTER TABLE thumbnail ADD COLUMN mimetype TEXT NOT NULL DEFAULT 'image/jpeg';