use std::fmt::{self, Display, Formatter};

#[derive(sqlx::Type, Debug, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[sqlx(type_name = "user_role")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
	Viewer,
	Editor,
	Admin,
}

impl Display for UserRole {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter.write_str(match (self, formatter.alternate()) {
			(Self::Viewer, true) => "Viewer",
			(Self::Viewer, false) => "viewer",
			(Self::Editor, true) => "Editor",
			(Self::Editor, false) => "editor",
			(Self::Admin, true) => "Admin",
			(Self::Admin, false) => "admin",
		})
	}
}

impl UserRole {
	pub fn display_options(self) -> impl Display + 'static {
		struct Helper(UserRole);
		impl Display for Helper {
			fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
				let role = self.0;
				for possibility in [UserRole::Viewer, UserRole::Editor, UserRole::Admin] {
					let selected = if role == possibility { " selected" } else { "" };
					write!(
						formatter,
						"<option value=\"{possibility}\"{selected}>{possibility:#}</option>"
					)?;
				}
				Ok(())
			}
		}

		Helper(self)
	}
}
