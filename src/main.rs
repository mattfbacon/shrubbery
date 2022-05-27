#![deny(
	absolute_paths_not_starting_with_crate,
	future_incompatible,
	keyword_idents,
	macro_use_extern_crate,
	meta_variable_misuse,
	missing_abi,
	missing_copy_implementations,
	non_ascii_idents,
	nonstandard_style,
	noop_method_call,
	pointer_structural_match,
	private_in_public,
	rust_2018_idioms
)]
#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::Extension;

mod config;
mod database;
mod error;
mod helpers;
mod percent;
mod routes;
mod server;
mod timestamp;
mod token;
mod viewspec;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("reading configuration: {0}")]
	Config(#[from] figment::Error),
	#[error("initializing logger: {0}")]
	Logger(#[from] log::SetLoggerError),
	#[error("connecting to database: {0}")]
	ConnectDb(#[from] sqlx::Error),
	#[error("running server: {0}")]
	RunServer(#[from] hyper::Error),
	#[error("binding to Unix socket at path {1}: {0}")]
	BindUnix(#[source] std::io::Error, std::path::PathBuf),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
	let config = config::config()?;
	let config = Arc::new(config);

	simple_logger::SimpleLogger::new()
		.with_level(config.log_level.external)
		.with_module_level(env!("CARGO_PKG_NAME"), config.log_level.internal)
		.init()?;

	let database = database::connect(&config.database_url)
		.await
		.map(Arc::new)?;

	let mut app = routes::configure();
	app = app.layer(Extension(database));
	app = app.layer(Extension(Arc::clone(&config)));

	log::info!("Listening on {}", config.address);
	server::run(app, &config.address).await
}
