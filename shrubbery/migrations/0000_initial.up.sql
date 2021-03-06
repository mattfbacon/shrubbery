CREATE TYPE user_role AS ENUM ('viewer', 'editor', 'admin');

CREATE TABLE users (
	id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	username VARCHAR NOT NULL UNIQUE,
	password VARCHAR NOT NULL,
	email VARCHAR,
	role user_role NOT NULL DEFAULT 'viewer',
	created_time TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
	last_login TIMESTAMP WITH TIME ZONE DEFAULT NULL
);

CREATE TYPE file_media_type AS ENUM ('image', 'video');

CREATE TABLE files (
	id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY, -- file is named by this ID
	name VARCHAR NOT NULL,
	description VARCHAR,
	media_type file_media_type NOT NULL
	-- timestamp data stored in filesystem
);

CREATE DOMAIN color AS char(7) CHECK (VALUE ~ '^#[0-9a-f]{6}$');

CREATE OR REPLACE FUNCTION require_created_by() RETURNS TRIGGER LANGUAGE PLPGSQL AS $func$ BEGIN
	ASSERT NEW.created_by IS NOT NULL, 'Cannot create this ' || TG_ARGV[0] || ' without a created_by user; created_by can only be NULL if the creator user is deleted after the ' || TG_ARGV[0] || 'is created';
	RETURN NEW;
END $func$;

CREATE TABLE tag_categories (
	id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	name VARCHAR NOT NULL UNIQUE,
	description VARCHAR,
	color color NOT NULL,
	created_time TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
	created_by INTEGER REFERENCES users ON DELETE SET NULL -- null = user was deleted
);
CREATE TRIGGER tag_categories_require_created_by_on_insertion BEFORE INSERT ON tag_categories FOR EACH ROW EXECUTE PROCEDURE require_created_by('tag category');

CREATE TABLE tags (
	id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	name VARCHAR NOT NULL,
	description VARCHAR,
	category INTEGER REFERENCES tag_categories ON DELETE SET NULL, -- null = no category or category was deleted
	created_time TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
	created_by INTEGER REFERENCES users ON DELETE SET NULL, -- null = user was deleted
	UNIQUE(name, category)
);
CREATE TRIGGER tags_require_created_by_on_insertion BEFORE INSERT ON tags FOR EACH ROW EXECUTE PROCEDURE require_created_by('tag');

CREATE TABLE file_tags (
	id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	file BIGINT NOT NULL REFERENCES files ON DELETE CASCADE,
	tag INTEGER NOT NULL REFERENCES tags ON DELETE CASCADE,
	UNIQUE(file, tag)
);
