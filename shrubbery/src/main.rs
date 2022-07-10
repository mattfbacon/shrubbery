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
	#[error("connecting to database: {0}")]
	ConnectDb(#[from] sqlx::Error),
	#[error("running server: {0}")]
	RunServer(#[from] hyper::Error),
	#[error("binding to Unix socket at path {1}: {0}")]
	BindUnix(#[source] std::io::Error, std::path::PathBuf),
	#[error("creating file storage directory due to it not existing at startup: {0}")]
	CreateFileStorage(#[source] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
	let config = config::config()?;

	if !config.file_storage.exists() {
		std::fs::create_dir_all(&config.file_storage).map_err(Error::CreateFileStorage)?;
	}

	let config = Arc::new(config);

	init_logging(config.log_level);

	let database = database::connect(&config.database_url)
		.await
		.map(Arc::new)?;

	let mut app = routes::configure();
	app = app.layer(Extension(database));
	app = app.layer(Extension(Arc::clone(&config)));
	app = app.layer(tower_http::trace::TraceLayer::new_for_http());

	tracing::info!(address = %config.address, "listening");
	server::run(app, &config.address).await
}

fn init_logging(log_level: config::LogLevel) {
	use tracing_subscriber::filter::FilterFn;
	use tracing_subscriber::layer::{Layer, SubscriberExt};
	use tracing_subscriber::util::SubscriberInitExt;

	let filter = FilterFn::new(move |metadata| {
		let required_level = match metadata.module_path() {
			Some(env!("CARGO_PKG_NAME")) => log_level.internal,
			_ => log_level.external,
		};
		// "If a Level is considered less than a LevelFilter, it should be considered enabled; if greater than or equal to the LevelFilter, that level is disabled."
		metadata.level() < &required_level
	});

	let layer = tracing_subscriber::fmt::layer()
		.with_file(true)
		.with_line_number(true)
		.with_writer(std::io::stderr);

	tracing_subscriber::registry()
		.with(layer.with_filter(filter))
		.init();
}
