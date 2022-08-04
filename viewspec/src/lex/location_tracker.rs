use super::span::Location;

#[derive(Debug)]
pub(super) struct LocationTracker<I> {
	inner: I,
	index: Location,
}

impl<I> LocationTracker<I> {
	pub(super) fn new(inner: I) -> Self {
		Self { inner, index: 0 }
	}
}

impl<T, I: Iterator<Item = T>> Iterator for LocationTracker<I> {
	type Item = (Location, T);

	fn next(&mut self) -> Option<(Location, T)> {
		let val = self.inner.next()?;
		let index = self.index;
		self.index += 1;
		Some((index, val))
	}
}
