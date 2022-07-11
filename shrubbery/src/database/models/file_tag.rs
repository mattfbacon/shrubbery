use ormx::Table;

use super::{FileId, TagId};

pub type Id = super::Id;
#[derive(Table)]
#[ormx(table = "file_tags", insertable, deletable)]
pub struct FileTag {
	pub id: Id,
	pub file: FileId,
	pub tag: TagId,
}
