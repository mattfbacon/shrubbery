DROP TABLE file_tags;

DROP TRIGGER tags_require_created_by_on_insertion ON tags;
DROP TABLE tags;

DROP TRIGGER tag_categories_require_created_by_on_insertion ON tag_categories;
DROP TABLE tag_categories;

DROP FUNCTION require_created_by;

DROP DOMAIN color;

DROP TABLE files;

DROP TABLE users;
DROP TYPE user_role;
