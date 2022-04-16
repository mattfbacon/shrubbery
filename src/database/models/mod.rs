use crate::timestamp::Timestamp;
use ormx::Table;

pub mod color;
pub mod user_crypt;

pub use color::Color;
pub use user_crypt::PasswordHash as UserPassword;

type Id = i32;
type BigId = i64;

pub type UserId = Id;
#[derive(Table)]
#[ormx(table = "users", id = id, insertable, deletable)]
pub struct User {
	#[ormx(default, get_optional = by_id(UserId))]
	pub id: UserId,
	#[ormx(get_optional = by_username(&str))]
	pub username: String,
	#[ormx(custom_type, by_ref)]
	pub password: UserPassword,
	pub email: Option<String>,
	#[ormx(default)]
	pub view_perm: bool,
	#[ormx(default)]
	pub edit_perm: bool,
	#[ormx(default)]
	pub admin_perm: bool,
	#[ormx(default)]
	pub created_time: Timestamp,
	#[ormx(default, set)]
	pub last_login: Option<Timestamp>,
}

pub type FileId = BigId;
#[derive(Table)]
#[ormx(table = "files", id = id, insertable, deletable)]
pub struct File {
	#[ormx(default)]
	pub id: FileId,
	pub name: String,
	pub description: Option<String>,
}

pub type TagCategoryId = Id;
#[derive(Table)]
#[ormx(table = "tag_categories", id = id, insertable, deletable)]
pub struct TagCategory {
	#[ormx(default)]
	pub id: TagCategoryId,
	pub name: String,
	pub description: Option<String>,
	#[ormx(custom_type)]
	pub color: Option<Color>,
	#[ormx(default)]
	pub created_time: Timestamp,
	pub created_by: Option<UserId>,
}

pub type TagId = Id;
#[derive(Table)]
#[ormx(table = "tags", id = id, insertable, deletable)]
pub struct Tag {
	#[ormx(default)]
	pub id: TagId,
	pub name: String,
	pub description: Option<String>,
	pub category: Option<TagCategoryId>,
	pub created_time: Timestamp,
	pub created_by: Option<UserId>,
}

pub type FileTagId = Id;
#[derive(Table)]
#[ormx(table = "file_tags", id = id, insertable, deletable)]
pub struct FileTag {
	#[ormx(default)]
	pub id: FileTagId,
	pub file: FileId,
	pub tag: TagId,
}
