use super::BigId;
use ormx::Table;

pub type FileId = BigId;

#[derive(sqlx::Type, Debug, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "file_media_type")]
#[sqlx(rename_all = "snake_case")]
pub enum MediaType {
	Image,
	Video,
}

#[derive(Table)]
#[ormx(table = "files", insertable, deletable)]
pub struct File {
	pub id: FileId,
	pub name: String,
	pub description: Option<String>,
	#[ormx(custom_type)]
	pub media_type: MediaType,
}
