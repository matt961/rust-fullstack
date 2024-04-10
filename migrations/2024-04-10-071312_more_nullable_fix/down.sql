-- This file should undo anything in `up.sql`
ALTER TABLE posts
ALTER COLUMN post_content
DROP NOT NULL;
