use chrono::{DateTime, Utc};

pub type Timestamp = DateTime<Utc>;

pub fn now() -> Timestamp {
	Utc::now()
}

pub fn is_in_past(t: &Timestamp) -> bool {
	t < &now()
}
