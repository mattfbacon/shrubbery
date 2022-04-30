use crate::timestamp::Timestamp;
use ormx::Table;

pub mod color;
pub mod user_crypt;
pub mod user_role;

pub use color::Color;
pub use user_crypt::PasswordHash as UserPassword;
pub use user_role::UserRole;

type Id = i32;
type BigId = i64;

pub type UserId = Id;
#[derive(Table)]
#[ormx(table = "users", insertable, deletable)]
pub struct User {
	#[ormx(get_optional = by_id(UserId))]
	pub id: UserId,
	#[ormx(get_optional = by_username(&str))]
	pub username: String,
	#[ormx(custom_type, by_ref)]
	pub password: UserPassword,
	pub email: Option<String>,
	#[ormx(default, custom_type, by_ref)]
	pub role: UserRole,
	#[ormx(default)]
	pub created_time: Timestamp,
	#[ormx(default, set)]
	pub last_login: Option<Timestamp>,
}

pub type FileId = BigId;
#[derive(Table)]
#[ormx(table = "files", insertable, deletable)]
pub struct File {
	pub id: FileId,
	pub name: String,
	pub description: Option<String>,
}

pub type TagCategoryId = Id;
#[derive(Table)]
#[ormx(table = "tag_categories", insertable, deletable)]
pub struct TagCategory {
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
#[ormx(table = "tags", insertable, deletable)]
pub struct Tag {
	pub id: TagId,
	pub name: String,
	pub description: Option<String>,
	pub category: Option<TagCategoryId>,
	pub created_time: Timestamp,
	pub created_by: Option<UserId>,
}

pub type FileTagId = Id;
#[derive(Table)]
#[ormx(table = "file_tags", insertable, deletable)]
pub struct FileTag {
	pub id: FileTagId,
	pub file: FileId,
	pub tag: TagId,
}
