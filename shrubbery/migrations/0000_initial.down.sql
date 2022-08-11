DROP TABLE file_tags;

DROP FUNCTION tag_by_category_and_name;
DROP FUNCTION tags_by_name;
DROP FUNCTION tags_by_category;
DROP TRIGGER tags_require_created_by_on_insertion ON tags;
DROP TABLE tags;

DROP FUNCTION tag_category_by_name;
DROP TRIGGER tag_categories_require_created_by_on_insertion ON tag_categories;
DROP TABLE tag_categories;

DROP FUNCTION require_created_by;

DROP DOMAIN color;

DROP TABLE files;
DROP TYPE file_media_type;

DROP TABLE users;
DROP TYPE user_role;
