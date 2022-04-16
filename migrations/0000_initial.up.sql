CREATE TABLE users (
	id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	username VARCHAR NOT NULL UNIQUE,
	password VARCHAR NOT NULL,
	email VARCHAR,
	view_perm BOOLEAN NOT NULL DEFAULT TRUE,
	edit_perm BOOLEAN NOT NULL DEFAULT FALSE,
	admin_perm BOOLEAN NOT NULL DEFAULT FALSE,
	created_time TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
	last_login TIMESTAMP WITH TIME ZONE DEFAULT NULL
);

CREATE TABLE files (
	id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY, -- file is named by this ID
	name VARCHAR NOT NULL,
	description VARCHAR
	-- timestamp data stored in filesystem
);

CREATE DOMAIN color AS char(7) CHECK (VALUE ~ '^#[0-9a-f]{6}$');

CREATE FUNCTION require_created_by() RETURNS TRIGGER LANGUAGE PLPGSQL AS $func$ BEGIN
	ASSERT NEW.created_by IS NOT NULL, 'Cannot create this ' || TG_ARGV[0] || ' without a created_by user; created_by can only be NULL if the creator user is deleted after the ' || TG_ARGV[0] || 'is created';
END $func$;

CREATE TABLE tag_categories (
	id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	name VARCHAR NOT NULL UNIQUE,
	description VARCHAR,
	color color,
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
	created_by INTEGER REFERENCES users ON DELETE SET NULL -- null = user was deleted
);
CREATE TRIGGER tags_require_created_by_on_insertion BEFORE INSERT ON tags FOR EACH ROW EXECUTE PROCEDURE require_created_by('tag');

CREATE TABLE file_tags (
	id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	file BIGINT NOT NULL REFERENCES files ON DELETE CASCADE,
	tag INTEGER NOT NULL REFERENCES tags ON DELETE CASCADE
);
