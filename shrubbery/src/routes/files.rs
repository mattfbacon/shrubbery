use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse, Redirect, Response};
use axum::{extract, Router};

use crate::config::Config;
use crate::database::models::media_type::MediaType as FileMediaType;
use crate::database::{models, Database};
use crate::error;
use crate::helpers::{auth, multipart};

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
			Self::Created => "Created the file",
			Self::Replaced => "Replaced the file",
			Self::Updated => "Updated the file",
			Self::UpdatedTags => "Updated tags for the file",
		}
	}
}

type TagsByCategory = HashMap<Option<String>, Vec<(models::TagId, String, bool)>>;

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

		let mut ret: TagsByCategory = HashMap::new();
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

pub async fn post_handler(
	auth::Auth(self_user): auth::Auth,
	extract::Path((file_id,)): extract::Path<(models::FileId,)>,
	mut multipart: extract::Multipart,
	extract::Extension(config): extract::Extension<Arc<Config>>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<Response, ErrorResponse> {
	let database = &*database;
	let action = match multipart.next_field().await.map_err(error::Multipart)? {
		Some(action) if action.name() == Some("action") => {
			action.text().await.map_err(error::Multipart)?
		}
		Some(_) => return Err(error::WrongFieldOrder("action").into()),
		None => return Err(error::ExpectedField("action").into()),
	};

	match action.as_str() {
		"update" => {
			#[derive(serde::Deserialize)]
			pub struct UpdateReq {
				name: String,
				description: Option<String>,
				media_type: models::MediaType,
			}

			let mut req: UpdateReq = multipart::deserialize_from_multipart(&mut multipart).await?;
			crate::helpers::set_none_if_empty(&mut req.description);
			let file = sqlx::query_as!(
				models::File,
				r#"UPDATE files SET name = $2, description = $3, media_type = $4 WHERE id = $1 RETURNING id, name, description, media_type as "media_type: _""#,
				file_id,
				req.name,
				req.description,
				req.media_type as _
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
		"update-tags" => {
			let file = models::File::by_id(database, file_id)
				.await
				.map_err(error::Sqlx)?
				.ok_or(error::EntityNotFound("file"))?;

			let mut new_tags = Vec::new();
			while let Some(field) = multipart.next_field().await.map_err(error::Multipart)? {
				if field.name() != Some("tags") {
					return Err(error::WrongFieldOrder("tags").into());
				}
				let tag_id_str = field.text().await.map_err(error::Multipart)?;
				let tag_id = tag_id_str
					.parse()
					.map_err(|_| error::BadRequest(Cow::Borrowed("tag ID is not valid number")))?;
				new_tags.push(tag_id);
			}

			let mut transaction = database.begin().await.map_err(error::Sqlx)?;
			sqlx::query!("DELETE FROM file_tags WHERE file = $1", file_id)
				.execute(&mut transaction)
				.await
				.map_err(error::Sqlx)?;
			sqlx::query!(
				"INSERT INTO file_tags (file, tag) (SELECT $1 as file, unnest as tag FROM unnest(cast($2 as bigint[])))",
				file_id,
				&new_tags
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
		"replace" => {
			let (media_type, write_state) = multipart::WriteToFile::start(&mut multipart, "file").await?;
			write_state.replace(file_id, &config).await?;
			let file = sqlx::query_as!(
				models::File,
				r#"UPDATE files SET media_type = $2 WHERE id = $1 RETURNING id, name, description, media_type as "media_type: _""#,
				file_id,
				media_type as _,
			)
			.fetch_optional(database)
			.await
			.map_err(error::Sqlx)?.ok_or_else(|| {
				log::warn!("file with ID {file_id} existed in filesystem but not in database");
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
		"delete" => {
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
		_ => Err(error::BadRequest(Cow::Borrowed("unknown action")).into()),
	}
}

pub fn configure() -> Router {
	let mut app = Router::new();

	app = app.route(
		"/:file_id",
		axum::routing::get(get_handler).post(post_handler),
	);

	app
}
