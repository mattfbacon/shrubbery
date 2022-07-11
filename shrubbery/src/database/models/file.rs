use ormx::Table;

use super::media_type::MediaType;

pub type Id = super::BigId;

#[derive(Table)]
#[ormx(table = "files", insertable, deletable)]
pub struct File {
	#[ormx(get_optional = by_id(Id))]
	pub id: Id,
	pub name: String,
	pub description: Option<String>,
	#[ormx(custom_type)]
	pub media_type: MediaType,
}
