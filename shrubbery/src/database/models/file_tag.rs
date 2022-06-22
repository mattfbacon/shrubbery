use ormx::Table;

use super::{FileId, Id, TagId};

pub type FileTagId = Id;
#[derive(Table)]
#[ormx(table = "file_tags", insertable, deletable)]
pub struct FileTag {
	pub id: FileTagId,
	pub file: FileId,
	pub tag: TagId,
}
