pub type PageNum = i64;

#[derive(serde::Deserialize, Clone, Copy)]
pub struct Query {
	#[serde(default = "default_page")]
	pub page: PageNum,
	#[serde(default = "default_page_size")]
	pub page_size: PageNum,
}

const fn default_page() -> PageNum {
	0
}

const fn default_page_size() -> PageNum {
	20
}

impl Query {
	pub fn offset(&self) -> PageNum {
		self.page * self.page_size
	}

	pub fn limit(&self) -> PageNum {
		self.page_size
	}
}

#[derive(askama::Template, Clone, Copy)]
#[template(path = "partials/pagination.html")]
pub struct Template {
	pub inner: Query,
	pub num_pages: PageNum,
}

impl Template {
	pub fn from_query(query: Query, num_pages: PageNum) -> Self {
		Self {
			inner: query,
			num_pages,
		}
	}

	pub fn href(&self) -> String {
		format!(
			"?page={}&page_size={}",
			self.inner.page, self.inner.page_size
		)
	}

	pub fn next_page(mut self) -> Option<Self> {
		self.inner.page = self.inner.page.checked_add(1)?;
		if self.inner.page < self.num_pages {
			Some(self)
		} else {
			None
		}
	}

	pub fn prev_page(mut self) -> Option<Self> {
		self.inner.page = self.inner.page.checked_sub(1)?;
		if self.inner.page >= 0 {
			Some(self)
		} else {
			None
		}
	}
}
