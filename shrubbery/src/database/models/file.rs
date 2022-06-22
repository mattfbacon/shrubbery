use ormx::Table;

use super::media_type::MediaType;
use super::BigId;

pub type FileId = BigId;

#[derive(Table)]
#[ormx(table = "files", insertable, deletable)]
pub struct File {
	#[ormx(get_optional = by_id(FileId))]
	pub id: FileId,
	pub name: String,
	pub description: Option<String>,
	#[ormx(custom_type)]
	pub media_type: MediaType,
}
