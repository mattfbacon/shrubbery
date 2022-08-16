//! Defines the [Error] type and a [Result] alias.

/// Errors that can occur while extracting multipart data.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// An underlying multipart error from `multer`.
	#[error("multipart error: {0}")]
	Multipart(#[from] multer::Error),
	/// Expected a field with the given name.
	#[error("expected {0:?} field")]
	ExpectedField(String),
	/// Expected a field but found the end of the multipart data.
	#[error("unexpected end of fields")]
	UnexpectedEnd,
	/// A custom error from an extractor, typically specific to that type.
	#[error("could not parse to {target}: {error}")]
	Custom {
		/// The type that was being extracted when the error occurred.
		target: &'static str,
		/// A textual description of the error that occurred.
		error: String,
	},
}

/// A convenience alias of `Result` where `E` defaults to [Error].
pub type Result<T, E = Error> = std::result::Result<T, E>;
