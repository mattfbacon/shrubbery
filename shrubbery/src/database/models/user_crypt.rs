use argon2::password_hash;
use password_hash::errors::Result as PwResult;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef, Postgres};
use sqlx::{Decode, Encode, Type};

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct PasswordHash {
	hash: String,
}

impl PasswordHash {
	pub fn hash(password: &str) -> PwResult<Self> {
		use argon2::PasswordHasher as _;
		let salt = password_hash::SaltString::generate(&mut password_hash::rand_core::OsRng);
		Ok(Self {
			hash: argon2::Argon2::default()
				.hash_password(password.as_bytes(), &salt)?
				.to_string(),
		})
	}

	pub fn verify(&self, password: &str) -> PwResult<bool> {
		use argon2::PasswordVerifier as _;
		match argon2::Argon2::default().verify_password(password.as_bytes(), &self.to_hash()) {
			Ok(()) => Ok(true),
			Err(password_hash::errors::Error::Password) => Ok(false),
			Err(other_err) => Err(other_err),
		}
	}

	fn to_hash(&self) -> argon2::PasswordHash<'_> {
		// the contents are guaranteed to be valid, but are stored as a String due to lifetime concerns
		argon2::PasswordHash::new(&self.hash).unwrap()
	}
}

/// create a `PasswordHash` object from an encoded representation of an existing hash
impl TryFrom<String> for PasswordHash {
	type Error = password_hash::Error;

	fn try_from(raw: String) -> Result<Self, Self::Error> {
		let _ = argon2::PasswordHash::new(&raw)?;
		Ok(Self { hash: raw })
	}
}

impl Type<Postgres> for PasswordHash {
	fn type_info() -> PgTypeInfo {
		<String as Type<Postgres>>::type_info()
	}

	fn compatible(other: &PgTypeInfo) -> bool {
		<String as Type<Postgres>>::compatible(other)
	}
}

impl Encode<'_, Postgres> for PasswordHash {
	fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> sqlx::encode::IsNull {
		<_ as Encode<'_, Postgres>>::encode_by_ref(&self.hash, buf)
	}
}

impl Decode<'_, Postgres> for PasswordHash {
	fn decode(value_ref: PgValueRef<'_>) -> Result<Self, sqlx::error::BoxDynError> {
		let raw = <String as Decode<'_, Postgres>>::decode(value_ref)?;
		Ok(Self::try_from(raw)?)
	}
}

impl super::User {
	#[inline]
	pub fn verify_password(&self, password: &str) -> PwResult<bool> {
		self.password.verify(password)
	}
}
