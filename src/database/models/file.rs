use super::BigId;
use ormx::Table;

pub type FileId = BigId;

#[derive(Table)]
#[ormx(table = "files", insertable, deletable)]
pub struct File {
	pub id: FileId,
	pub name: String,
	pub description: Option<String>,
}
