use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Result;

pub mod models;

pub type Database = PgPool;

pub async fn connect(conn_str: &str) -> Result<Database> {
	let conn = PgPoolOptions::new()
		.max_connections(5)
		.connect(conn_str)
		.await?;
	sqlx::migrate!().run(&conn).await?;
	Ok(conn)
}
