use std::fmt::{self, Display, Formatter};

#[derive(sqlx::Type, Debug, serde::Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "file_media_type")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
	Image,
	Video,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid media type; expected `image` or `video`")]
pub struct FromStrError;

impl std::str::FromStr for MediaType {
	type Err = FromStrError;

	fn from_str(raw: &str) -> Result<Self, Self::Err> {
		match raw {
			"image" | "Image" => Ok(Self::Image),
			"video" | "Video" => Ok(Self::Video),
			_ => Err(FromStrError),
		}
	}
}

axum_easy_multipart::impl_for_from_str!(MediaType);

impl Display for MediaType {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		use MediaType::*;
		formatter.write_str(match (self, formatter.alternate()) {
			(Image, true) => "Image",
			(Image, false) => "image",
			(Video, true) => "Video",
			(Video, false) => "video",
		})
	}
}

impl MediaType {
	pub fn display_options(value: Option<Self>) -> impl Display + 'static {
		struct Helper(Option<MediaType>);
		impl Display for Helper {
			fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
				let value = self.0;
				for possibility in [MediaType::Image, MediaType::Video] {
					let selected = if value == Some(possibility) {
						" selected"
					} else {
						""
					};
					write!(
						formatter,
						"<option value=\"{possibility}\"{selected}>{possibility:#}</option>"
					)?;
				}
				Ok(())
			}
		}

		Helper(value)
	}

	pub fn from_mime(mime: mime::Mime) -> Option<Self> {
		match mime.type_() {
			mime::IMAGE => Some(Self::Image),
			mime::VIDEO => Some(Self::Video),
			_ => None,
		}
	}
}
