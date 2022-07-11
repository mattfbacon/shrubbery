use crate::database::models;

pub async fn get_tag_categories_lean(
	database: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
) -> sqlx::Result<Vec<(models::TagCategoryId, String)>> {
	sqlx::query!("SELECT id, name FROM tag_categories")
		.map(|record| (record.id, record.name))
		.fetch_all(database)
		.await
}

pub fn display_category_options(
	current: Option<models::TagCategoryId>,
	categories: &[(models::TagCategoryId, String)],
) -> impl std::fmt::Display + '_ {
	struct Helper<'a> {
		current: Option<models::TagCategoryId>,
		categories: &'a [(models::TagCategoryId, String)],
	}

	impl std::fmt::Display for Helper<'_> {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			#[inline]
			fn selected(is_selected: bool) -> &'static str {
				if is_selected {
					" selected"
				} else {
					""
				}
			}

			write!(
				f,
				r#"<option value="null"{}>(none)</option>"#,
				selected(self.current.is_none())
			)?;
			for &(category_id, ref category_name) in self.categories {
				write!(
					f,
					r#"<option value="{}"{}>{}</option>"#,
					category_id,
					selected(self.current == Some(category_id)),
					askama::filters::escape(askama_escape::Html, &category_name).unwrap() // always Ok
				)?;
			}
			Ok(())
		}
	}

	Helper {
		current,
		categories,
	}
}
