#![feature(type_alias_impl_trait)]
#![deny(private_in_public, rust_2018_idioms)]

use actix_web::{middleware as mid, web, App as ActixApp, HttpServer};
use anyhow::Context as _;
use std::sync::Arc;

mod config;
mod database;
mod helpers;
mod percent;
mod routes;
mod timestamp;
mod token;

use config::BindableAddr;

async fn main_() -> anyhow::Result<()> {
	let config = config::config().context("Reading configuration")?;
	let config = Arc::new(config);

	simple_logger::SimpleLogger::new()
		.with_level(config.log_level.external)
		.with_module_level(env!("CARGO_PKG_NAME"), config.log_level.internal)
		.init()
		.context("Initializing logging")?;

	let database = database::connect(&config.database_url)
		.await
		.context("Initializing database manager")
		.map(Arc::new)?;

	let mut http = {
		let config = Arc::clone(&config);
		HttpServer::new(move || {
			ActixApp::new()
				.app_data(web::Data::from(Arc::clone(&config)))
				.app_data(web::Data::from(Arc::clone(&database)))
				.wrap(mid::NormalizePath::trim())
				.wrap(routes::error_handlers())
				.configure(routes::configure)
				.service(actix_files::Files::new("/", "static"))
				.wrap(mid::Logger::default())
		})
	};
	if let Some(num_workers) = config.num_workers {
		http = http.workers(num_workers);
	}

	log::info!("Listening on {}", config.address);
	match &config.address {
		BindableAddr::Tcp(addr) => http.bind(addr),
		BindableAddr::Unix(path) => http.bind_uds(path),
	}
	.context("Binding server to address")?
	.run()
	.await
	.context("Running server")
}

fn main() -> anyhow::Result<()> {
	actix_web::rt::System::new().block_on(main_())
}
