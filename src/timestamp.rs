use chrono::Utc;

pub type Timestamp = chrono::DateTime<Utc>;
pub type Date = chrono::Date<Utc>;
pub type Time = chrono::NaiveTime;

pub fn now() -> Timestamp {
	Utc::now()
}

pub fn is_in_past(t: &Timestamp) -> bool {
	t < &now()
}
