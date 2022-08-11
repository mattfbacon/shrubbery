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

CREATE FUNCTION tag_category_by_name(desired_name tag_categories.name%TYPE) RETURNS tag_categories.id%TYPE RETURNS NULL ON NULL INPUT STABLE LANGUAGE plpgsql AS $func$
	DECLARE id tag_categories.id%TYPE;
	BEGIN
		SELECT tag_categories.id INTO id FROM tag_categories WHERE tag_categories.name = desired_name;
		IF id IS NULL THEN
			RAISE EXCEPTION using message = 'unknown tag category', detail = desired_name;
		END IF;
		RETURN id;
	END
$func$;

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

-- it is not considered an error for there to be no tags in a category
CREATE FUNCTION tags_by_category(desired_category tag_categories.name%TYPE) RETURNS table(id tags.id%TYPE) STABLE LANGUAGE SQL AS 'SELECT tags.id FROM tags WHERE tags.category = tag_category_by_name(desired_category)';
CREATE FUNCTION tags_by_name(desired_name tags.name%TYPE) RETURNS table(id tags.id%TYPE) STABLE LANGUAGE plpgsql AS $func$ BEGIN
	CREATE TEMP TABLE ids ON COMMIT DROP AS SELECT tags.id FROM tags WHERE tags.name = desired_name;
	IF count(*) = 0 FROM ids THEN -- https://www.postgresql.org/docs/14/plpgsql-expressions.html
		RAISE EXCEPTION using message = 'no tags by name', detail = desired_name;
	END IF;
	SELECT id FROM ids;
END $func$;
CREATE FUNCTION tag_by_category_and_name(desired_category tag_categories.name%TYPE, desired_name tags.name%TYPE) RETURNS tags.id%TYPE STABLE LANGUAGE plpgsql AS $func$
	DECLARE id tags.id%TYPE;
	BEGIN
		ASSERT desired_name IS NOT NULL, 'tag name is null';
		SELECT tags.id INTO id FROM tags WHERE tags.name = desired_name AND tags.category IS NOT DISTINCT FROM tag_category_by_name(desired_category);
		IF id IS NULL THEN
			RAISE EXCEPTION using message = 'unknown tag', detail = desired_category, hint = desired_name; -- abusing exception fields
		END IF;
		RETURN id;
	END
$func$;

CREATE TABLE file_tags (
	id INTEGER PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	file BIGINT NOT NULL REFERENCES files ON DELETE CASCADE,
	tag INTEGER NOT NULL REFERENCES tags ON DELETE CASCADE,
	UNIQUE(file, tag)
);
