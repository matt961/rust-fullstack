CREATE EXTENSION pg_uuidv7; -- using ghcr.io/fboulnois/pg_uuidv7 pg image on dev

CREATE TABLE posts (
	id uuid not null default uuid_generate_v7() PRIMARY KEY,
	user_id int not null references users(id),
	post_content text,
	tags varchar(52)[] default '{}'
);

CREATE INDEX ix_post_user_id ON posts(user_id);

CREATE INDEX ix_gist_post_tags ON posts(tags) USING gin;
