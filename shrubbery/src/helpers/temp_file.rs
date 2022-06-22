use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tokio::fs::File;

pub struct TempFile<'a> {
	final_path: &'a Path,
	temp_path: Option<PathBuf>,
	file: File,
}

impl<'a> TempFile<'a> {
	/// The path is expected to have a filename, and that filename is expected to be valid UTF-8.
	pub async fn create(final_path: &'a Path) -> Result<TempFile<'a>, std::io::Error> {
		let temp_timestamp = SystemTime::now()
			.duration_since(SystemTime::UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let temp_path = final_path.with_file_name(format!(
			"{}.temp.{}",
			final_path.file_name().unwrap().to_str().unwrap(),
			temp_timestamp
		));
		let file = File::create(&temp_path).await?;
		Ok(Self {
			final_path,
			temp_path: Some(temp_path),
			file,
		})
	}
}

impl TempFile<'_> {
	pub async fn move_into_place(mut self) -> Result<(), std::io::Error> {
		let temp_path = self.temp_path.take().unwrap();
		tokio::fs::rename(&temp_path, self.final_path).await
	}
}

impl AsRef<File> for TempFile<'_> {
	fn as_ref(&self) -> &File {
		&self.file
	}
}

impl AsMut<File> for TempFile<'_> {
	fn as_mut(&mut self) -> &mut File {
		&mut self.file
	}
}

impl Drop for TempFile<'_> {
	fn drop(&mut self) {
		if let Some(ref temp_path) = self.temp_path {
			let _ = std::fs::remove_file(temp_path);
		}
	}
}
