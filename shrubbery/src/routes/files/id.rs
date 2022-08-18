use std::collections::BTreeMap;
use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse, Redirect, Response};
use axum::{extract, Router};

use crate::config::Config;
use crate::database::models::media_type::MediaType as FileMediaType;
use crate::database::{models, Database};
use crate::error;
use crate::helpers::auth;

#[derive(Clone, Copy)]
enum Action {
	Created,
	Replaced,
	Updated,
	UpdatedTags,
}
impl Action {
	fn as_message(self) -> &'static str {
		match self {
			Self::Created => "Created",
			Self::Replaced => "Replaced",
			Self::Updated => "Updated",
			Self::UpdatedTags => "Updated tags for",
		}
	}
}

type TagsByCategory = BTreeMap<Option<String>, Vec<(models::TagId, String, bool)>>;

#[derive(askama::Template)]
#[template(path = "files/page.html")]
struct Template {
	self_user: models::User,
	file: models::File,
	action: Option<Action>,
	tags_by_category: TagsByCategory,
}
crate::helpers::impl_into_response!(Template);

impl Template {
	async fn get_tags_by_category(
		database: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
		file_id: models::FileId,
	) -> sqlx::Result<TagsByCategory> {
		use futures::TryStreamExt as _;

		let mut ret: TagsByCategory = BTreeMap::new();
		let mut stream = sqlx::query!(r#"SELECT tags.id, tags.name, tag_categories.name as "category?", (SELECT count(*) > 0 FROM file_tags WHERE tag = tags.id AND file = $1) as "present!" FROM tags LEFT JOIN tag_categories ON tags.category = tag_categories.id ORDER BY category NULLS FIRST, name"#, file_id).fetch(database);
		while let Some(record) = stream.try_next().await? {
			let tags = ret.entry(record.category).or_insert(Vec::new());
			tags.push((record.id, record.name, record.present));
		}
		Ok(ret)
	}
}

#[derive(serde::Deserialize)]
pub struct Query {
	pub direct: Option<String>,
	pub created: Option<String>,
}

pub async fn get_handler(
	auth::Auth(self_user): auth::Auth,
	extract::Path((file_id,)): extract::Path<(models::FileId,)>,
	extract::Query(Query { direct, created }): extract::Query<Query>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
	extract::Extension(config): extract::Extension<Arc<Config>>,
	req_parts: http::request::Parts,
) -> Result<Response, ErrorResponse> {
	let database = &*database;

	if direct.is_some() {
		use tower::Service as _;
		let fs_path = config.file_storage.join(file_id.to_string());
		let mut service = tower_http::services::ServeFile::new(fs_path);
		let request = http::Request::from_parts(req_parts, ());
		let response = service.call(request).await;
		let mut response = response.map_err(|err| error::Io("serving file directly", err))?;
		response.headers_mut().remove("Content-Type");
		Ok(response.map(|body| {
			use http_body::Body as _;
			body.map_err(axum::Error::new).boxed_unsync()
		}))
	} else {
		let file = models::File::by_id(database, file_id)
			.await
			.map_err(error::Sqlx)?
			.ok_or(error::EntityNotFound("file"))?;
		Ok(
			Template {
				self_user,
				file,
				action: created.map(|_| Action::Created),
				tags_by_category: Template::get_tags_by_category(database, file_id)
					.await
					.map_err(error::Sqlx)?,
			}
			.into_response(),
		)
	}
}

pub struct MakeTempfile(Arc<Config>);

impl axum_easy_multipart::file::MakeTempfile for MakeTempfile {
	fn extract_from_extensions(extensions: &http::Extensions) -> Self {
		Self(Arc::clone(extensions.get::<Arc<Config>>().unwrap()))
	}

	fn tempfile(&self) -> std::io::Result<tempfile::NamedTempFile> {
		tempfile::Builder::new().tempfile_in(&self.0.file_storage)
	}
}

#[derive(Debug, axum_easy_multipart::FromMultipart)]
#[multipart(tag = "action")]
pub enum PostRequest {
	#[multipart(rename = "delete")]
	Delete {},
	#[multipart(rename = "replace")]
	Replace {
		file: axum_easy_multipart::file::File<MakeTempfile>,
	},
	#[multipart(rename = "update")]
	Update {
		name: String,
		description: Option<String>,
		media_type: models::MediaType,
	},
	#[multipart(rename = "update-tags")]
	UpdateTags { tags: Vec<models::TagId> },
}

async fn post_delete_handler(
	file_id: models::FileId,
	config: &Config,
	database: &sqlx::Pool<sqlx::Postgres>,
) -> Result<Response, ErrorResponse> {
	tokio::fs::remove_file(config.file_storage.join(file_id.to_string()))
		.await
		.map_err(|err| error::Io("deleting file", err))?;
	let q_result = sqlx::query!("DELETE FROM files WHERE id = $1", file_id)
		.execute(database)
		.await
		.map_err(error::Sqlx)?;
	if q_result.rows_affected() == 0 {
		Err(error::EntityNotFound("file").into())
	} else {
		Ok(Redirect::to("/").into_response())
	}
}

async fn post_replace_handler(
	self_user: models::User,
	file_id: models::FileId,
	temp_file: axum_easy_multipart::file::File<impl axum_easy_multipart::file::MakeTempfile>,
	config: &Config,
	database: &sqlx::Pool<sqlx::Postgres>,
) -> Result<Response, ErrorResponse> {
	temp_file
		.temp_path
		.persist(config.file_storage.join(format!("{file_id}")))
		.map_err(|error| error::Io("replacing file in filesystem", error.error))?;
	let media_type = temp_file
		.content_type
		.as_ref()
		.and_then(models::MediaType::from_mime)
		.ok_or(error::BadContentType)?;
	let file = sqlx::query_as!(
		models::File,
		r#"UPDATE files SET media_type = $2 WHERE id = $1 RETURNING id, name, description, media_type as "media_type: models::MediaType""#,
		file_id,
		media_type as _,
	)
		.fetch_optional(database)
		.await
		.map_err(error::Sqlx)?.ok_or_else(|| {
			tracing::warn!("file with ID {file_id} existed in filesystem but not in database");
			error::EntityNotFound("file")
		})?;
	Ok(
		Template {
			self_user,
			file,
			action: Some(Action::Replaced),
			tags_by_category: Template::get_tags_by_category(database, file_id)
				.await
				.map_err(error::Sqlx)?,
		}
		.into_response(),
	)
}

async fn post_update_handler(
	self_user: models::User,
	file_id: models::FileId,
	name: String,
	mut description: Option<String>,
	media_type: models::MediaType,
	database: &sqlx::Pool<sqlx::Postgres>,
) -> Result<Response, ErrorResponse> {
	crate::helpers::set_none_if_empty(&mut description);
	let file = sqlx::query_as!(
		models::File,
		r#"UPDATE files SET name = $2, description = $3, media_type = $4 WHERE id = $1 RETURNING id, name, description, media_type as "media_type: models::MediaType""#,
		file_id,
		name,
		description,
		media_type as _
	)
	.fetch_optional(database)
	.await.map_err(error::Sqlx)?.ok_or(error::EntityNotFound("file"))?;
	Ok(
		Template {
			self_user,
			file,
			action: Some(Action::Updated),
			tags_by_category: Template::get_tags_by_category(database, file_id)
				.await
				.map_err(error::Sqlx)?,
		}
		.into_response(),
	)
}

async fn post_update_tags_handler(
	self_user: models::User,
	file_id: models::FileId,
	tags: Vec<models::TagId>,
	database: &sqlx::Pool<sqlx::Postgres>,
) -> Result<Response, ErrorResponse> {
	// get it now to return early if it doesn't exist
	let file = models::File::by_id(database, file_id)
		.await
		.map_err(error::Sqlx)?
		.ok_or(error::EntityNotFound("file"))?;

	let mut transaction = database.begin().await.map_err(error::Sqlx)?;
	sqlx::query!("DELETE FROM file_tags WHERE file = $1", file_id)
		.execute(&mut transaction)
		.await
		.map_err(error::Sqlx)?;
	sqlx::query!(
		"INSERT INTO file_tags (file, tag) (SELECT $1 as file, unnest as tag FROM unnest(cast($2 as int[])))",
		file_id,
		&tags
	)
	.execute(&mut transaction)
	.await
	.map_err(error::Sqlx)?;
	transaction.commit().await.map_err(error::Sqlx)?;

	Ok(
		Template {
			self_user,
			file,
			action: Some(Action::UpdatedTags),
			tags_by_category: Template::get_tags_by_category(database, file_id)
				.await
				.map_err(error::Sqlx)?,
		}
		.into_response(),
	)
}

pub async fn post_handler(
	auth::Auth(self_user): auth::Auth,
	extract::Path((file_id,)): extract::Path<(models::FileId,)>,
	axum_easy_multipart::Extractor(req): axum_easy_multipart::Extractor<PostRequest>,
	extract::Extension(config): extract::Extension<Arc<Config>>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<Response, ErrorResponse> {
	let database = &*database;

	match req {
		PostRequest::Delete {} => post_delete_handler(file_id, &config, database).await,
		PostRequest::Replace { file: temp_file } => {
			post_replace_handler(self_user, file_id, temp_file, &config, database).await
		}
		PostRequest::Update {
			name,
			description,
			media_type,
		} => post_update_handler(self_user, file_id, name, description, media_type, database).await,
		PostRequest::UpdateTags { tags } => {
			post_update_tags_handler(self_user, file_id, tags, database).await
		}
	}
}

pub fn configure() -> Router {
	let mut router = Router::new();

	router = router.route("/", axum::routing::get(get_handler).post(post_handler));

	router
}
