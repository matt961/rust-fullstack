-- This file should undo anything in `up.sql`
ALTER TABLE posts
ALTER COLUMN tags
DROP NOT NULL;
