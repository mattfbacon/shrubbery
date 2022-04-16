use crate::token::Key as TokenKey;
use log::LevelFilter;
use serde::Deserialize;

pub mod bindable;
pub use bindable::BindableAddr;

#[derive(Deserialize)]
pub struct Config {
	pub address: BindableAddr,
	#[serde(default = "default_log_level")]
	pub log_level: LogLevel,
	pub num_workers: Option<usize>,
	pub database_url: String,
	#[serde(default = "default_cookie_signing_key")]
	pub cookie_signing_key: TokenKey,
}

fn deserialize_level_filter<'de, D: serde::de::Deserializer<'de>>(
	d: D,
) -> Result<LevelFilter, D::Error>
where
	D::Error: serde::de::Error,
{
	String::deserialize(d)?
		.parse()
		.map_err(serde::de::Error::custom)
}

#[derive(Deserialize)]
#[serde(from = "LogLevelSerdeHelper")]
pub struct LogLevel {
	pub internal: LevelFilter,
	pub external: LevelFilter,
}

const fn default_log_level_internal() -> LevelFilter {
	LevelFilter::Info
}

const fn default_log_level_external() -> LevelFilter {
	LevelFilter::Warn
}

#[derive(Deserialize)]
#[serde(untagged)]
enum LogLevelSerdeHelper {
	#[serde(deserialize_with = "deserialize_level_filter")]
	Together(LevelFilter),
	Separate {
		#[serde(
			deserialize_with = "deserialize_level_filter",
			default = "default_log_level_internal"
		)]
		internal: LevelFilter,
		#[serde(
			deserialize_with = "deserialize_level_filter",
			default = "default_log_level_external"
		)]
		external: LevelFilter,
	},
}

impl From<LogLevelSerdeHelper> for LogLevel {
	fn from(helper: LogLevelSerdeHelper) -> Self {
		match helper {
			LogLevelSerdeHelper::Together(level) => Self {
				internal: level,
				external: level,
			},
			LogLevelSerdeHelper::Separate { internal, external } => Self { internal, external },
		}
	}
}

const fn default_log_level() -> LogLevel {
	LogLevel {
		internal: default_log_level_internal(),
		external: default_log_level_external(),
	}
}

fn default_cookie_signing_key() -> TokenKey {
	let generated = TokenKey::generate();
	let encoded = base64::encode(generated.as_raw_data());
	// print warning with `eprintln!` since logging is not initialized when config is loaded
	eprintln!("Warning: since you did not provide a cookie signing key, one was generated for you.");
	eprintln!("To avoid cookie signature errors when you restart the server, please add the following to your `shrubbery.toml`:");
	eprintln!("cookie_signing_key={:?}", encoded);
	generated
}

pub fn config() -> Result<Config, figment::Error> {
	use figment::providers::Format as _;

	figment::Figment::new()
		.merge(figment::providers::Toml::file("shrubbery.toml"))
		.merge(figment::providers::Env::prefixed("SHRUBBERY_"))
		.extract()
}
