use aes_gcm::aead::NewAead as _;
use aes_gcm::{Aes256Gcm, Nonce};
use cookie::Cookie;

#[derive(Debug, thiserror::Error)]
pub enum EncryptError {
	#[error("serializing token data: {0}")]
	Serialize(#[from] bincode::Error),
	#[error("encrypting serialized data: {0}")]
	Encrypt(#[from] aes_gcm::aead::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DecryptError {
	#[error("decoding base64 data: {0}")]
	Base64(#[from] base64::DecodeError),
	#[error("decoded data is too short")]
	TooShort,
	#[error("decrypting decoded data: {0}")]
	Decrypt(#[from] aes_gcm::aead::Error),
	#[error("deserializing decrypted data: {0}")]
	Deserialize(#[from] bincode::Error),
}

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
static ASSOCIATED_DATA: &[u8] = b"token";

pub struct Key([u8; KEY_LEN]);

impl Key {
	pub fn generate() -> Self {
		use rand::Rng as _;
		Self(rand::thread_rng().gen())
	}

	pub fn as_raw_data(&self) -> &[u8] {
		&self.0
	}
}

impl serde::Serialize for Key {
	fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		base64::encode(&self.0).serialize(serializer)
	}
}

impl<'de> serde::Deserialize<'de> for Key {
	fn deserialize<D: serde::de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
	where
		D::Error: serde::de::Error,
	{
		use serde::de::Error;
		let deserialized = <std::borrow::Cow<'_, str>>::deserialize(deserializer)?;
		let decoded = base64::decode(deserialized.as_bytes()).map_err(Error::custom)?;
		Ok(Self(decoded.as_slice().try_into().map_err(Error::custom)?))
	}
}

impl super::Token {
	pub fn encrypt_to_cookie(
		&self,
		key: &Key,
		keep_logged_in: bool,
	) -> Result<Cookie<'static>, EncryptError> {
		use aes_gcm::aead::AeadInPlace as _;
		use rand::RngCore as _;

		let serialized_size = bincode::serialized_size(self)?.try_into().unwrap();
		// 0. create buffer with desired size and generate and insert nonce
		let mut encrypted = vec![0; NONCE_LEN + serialized_size + TAG_LEN];
		let (nonce, in_out) = encrypted.split_at_mut(NONCE_LEN);
		let (in_out, tag_buffer) = in_out.split_at_mut(serialized_size);
		rand::thread_rng().fill_bytes(nonce);
		// 1. encode the data to Bincode format to the buffer
		bincode::serialize_into(&mut *in_out, self)?;
		// 2. encrypt the data with AES-GCM (this is based on the `cookie` crate's `PrivateJar::encrypt_cookie` method, but uses AES-GCM-SIV instead of AES-GCM)
		let tag = Aes256Gcm::new((&key.0).into()).encrypt_in_place_detached(
			Nonce::from_slice(nonce),
			ASSOCIATED_DATA,
			in_out,
		)?;
		tag_buffer.copy_from_slice(&tag);
		// 3. encode the encrypted and signed data with base64
		let encoded = base64::encode(&encrypted);
		// 4. create cookie with the proper options
		Ok(
			Cookie::build("token", encoded)
				.expires(if keep_logged_in {
					Some(std::time::SystemTime::from(self.expires_at).into())
				} else {
					// if there is no expiration or max age, the cookie will be removed at the end of the session
					None
				})
				.http_only(true)
				.same_site(cookie::SameSite::Strict)
				.finish(),
		)
	}

	pub fn decrypt(encrypted_and_encoded: &str, key: &Key) -> Result<Option<Self>, DecryptError> {
		use aes_gcm::aead::Aead as _;
		use aes_gcm::aead::Payload;

		let encrypted = base64::decode(encrypted_and_encoded)?;
		if encrypted.len() < NONCE_LEN {
			return Err(DecryptError::TooShort);
		}
		let (nonce, encrypted) = encrypted.split_at(NONCE_LEN);
		let decrypted = Aes256Gcm::new((&key.0).into()).decrypt(
			Nonce::from_slice(nonce),
			Payload {
				msg: encrypted,
				aad: ASSOCIATED_DATA,
			},
		)?;
		let ret: Self = bincode::deserialize(&decrypted)?;
		Ok(if ret.is_expired() { None } else { Some(ret) })
	}
}

#[cfg(test)]
mod test {
	use super::super::Token;
	use super::Key;
	#[test]
	fn encryption_roundtrip() {
		let key = Key::generate();
		let original = Token::new(0);
		let encrypted_cookie = original.encrypt_to_cookie(&key, false).unwrap();
		assert_eq!(encrypted_cookie.name(), "token");
		let decrypted = Token::decrypt(encrypted_cookie.value(), &key)
			.unwrap()
			.unwrap();
		// timestamp is rounded to seconds, so don't compare
		assert_eq!(original.user_id, decrypted.user_id);
	}
}
