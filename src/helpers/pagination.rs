pub type PageNum = i64;

#[derive(serde::Deserialize, Clone, Copy)]
pub struct Query {
	pub page: Option<PageNum>,
	pub page_size: Option<PageNum>,
}

pub const fn default_page() -> PageNum {
	0
}

pub const fn default_page_size() -> PageNum {
	20
}

impl Query {
	#[inline]
	pub fn page(&self) -> PageNum {
		self.page.unwrap_or(default_page())
	}

	#[inline]
	pub fn page_size(&self) -> PageNum {
		self.page_size.unwrap_or(default_page_size())
	}

	#[inline]
	pub fn offset(&self) -> PageNum {
		self.page() * self.page_size()
	}

	#[inline]
	pub fn limit(&self) -> PageNum {
		self.page_size()
	}
}

#[derive(askama::Template, Clone, Copy)]
#[template(path = "_partials/pagination.html")]
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

	pub fn href(&self) -> std::borrow::Cow<'static, str> {
		match (self.inner.page, self.inner.page_size) {
			(Some(page), Some(page_size)) => format!("?page={page}&page_size={page_size}"),
			(Some(page), None) => format!("?page={page}"),
			(None, Some(page_size)) => format!("?page_size={page_size}"),
			(None, None) => return "".into(),
		}
		.into()
	}

	pub fn next_page(mut self) -> Option<Self> {
		let new_page = self.inner.page().checked_add(1)?;
		self.inner.page = Some(new_page);
		if new_page < self.num_pages {
			Some(self)
		} else {
			None
		}
	}

	pub fn prev_page(mut self) -> Option<Self> {
		let new_page = self.inner.page().checked_sub(1)?;
		self.inner.page = Some(new_page);
		if new_page >= 0 {
			Some(self)
		} else {
			None
		}
	}
}
