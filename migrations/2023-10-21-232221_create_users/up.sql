CREATE TABLE users (
id serial not null,
email varchar(320) not null,

PRIMARY KEY (id), UNIQUE(email)
)
