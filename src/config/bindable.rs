use std::fmt::{self, Display, Formatter};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug)]
pub enum BindableAddr {
	Unix(PathBuf),
	Tcp(SocketAddr),
}

#[derive(Debug)]
pub enum BindableAddrFromStrError {
	UnknownProtocol(String),
	SocketAddr(<SocketAddr as FromStr>::Err),
}

impl Display for BindableAddrFromStrError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::UnknownProtocol(unknown) => write!(f, "unknown protocol {:?}", unknown),
			Self::SocketAddr(inner) => write!(f, "could not parse socket address: {}", inner),
		}
	}
}

use std::str::FromStr;
impl FromStr for BindableAddr {
	type Err = BindableAddrFromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let (protocol, inner) = s.split_once("://").unwrap_or(("tcp", s));
		match protocol {
			"unix" => Ok(Self::Unix(PathBuf::from(inner))),
			"tcp" => SocketAddr::from_str(inner)
				.map_err(Self::Err::SocketAddr)
				.map(Self::Tcp),
			unknown => Err(Self::Err::UnknownProtocol(unknown.to_owned())),
		}
	}
}

impl Display for BindableAddr {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Tcp(inner) => write!(f, "tcp://{}", inner),
			Self::Unix(inner) => write!(f, "unix://{}", inner.display()),
		}
	}
}

use serde::ser::{Serialize, Serializer};
impl Serialize for BindableAddr {
	fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
		self.to_string().serialize(s)
	}
}

use serde::de::{Deserialize, Deserializer, Error as DError};
impl<'de> Deserialize<'de> for BindableAddr {
	fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>
	where
		D::Error: DError,
	{
		String::deserialize(d)?.parse().map_err(DError::custom)
	}
}
