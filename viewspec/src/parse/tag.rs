//! Provides owned [`Tag`] and borrowed [`Ref`].
#![allow(unsafe_code)]

/// The basic node of the AST, referring to a tag within the database that files can be tagged with.
///
/// Can have a name and/or a category.
pub struct Tag(Box<TagInner>);

impl Tag {
	/// The maximum length of a category, in bytes.
	pub const MAX_CATEGORY_LEN: usize = u16::MAX as usize;

	/// Create a [`Tag`] that contains a `name`.
	///
	/// Analogous to `Ref::Name`.
	#[must_use]
	pub fn name(name: &str) -> Self {
		Self(TagInner::new(&[name], TagKind::Name))
	}

	/// Create a [`Tag`] that contains a `category`.
	///
	/// Analogous to `Ref::Category`.
	#[must_use]
	pub fn category(category: &str) -> Self {
		Self(TagInner::new(&[category], TagKind::Category))
	}

	/// Create a [`Tag`] that contains both a `category` and a `name`.
	///
	/// Analogous to `Ref::Both`.
	///
	/// Returns `None` if `category` is too long.
	#[must_use]
	pub fn both(category: &str, name: &str) -> Option<Self> {
		Some(Self(TagInner::new(
			&[category, name],
			TagKind::Both {
				name_start: category.len().try_into().ok()?,
			},
		)))
	}

	/// Get a view of this [`Tag`] as an instance of the [`Ref`] enum, which can then be matched on.
	///
	/// This is not provided as an implementation of `AsRef` because that is a reference-to-reference conversion and this only returns a type *containing* references.
	///
	/// It is also not provided as an implementation of `From` because that is an owned-to-owned conversion and provides no way to modify the return type based on the lifetime of the `&self` argument.
	#[must_use]
	pub fn as_ref(&self) -> Ref<'_> {
		match self.0.kind {
			TagKind::Name => Ref::Name(&self.0.data),
			TagKind::Category => Ref::Category(&self.0.data),
			TagKind::Both { name_start } => {
				let name_start = usize::from(name_start);
				let (category, name) = self.0.data.split_at(name_start);
				Ref::Both { category, name }
			}
		}
	}
}

impl Clone for Tag {
	fn clone(&self) -> Self {
		self.as_ref().to_owned()
	}
}

impl PartialEq for Tag {
	fn eq(&self, other: &Self) -> bool {
		self.as_ref() == other.as_ref()
	}
}

impl Eq for Tag {}

impl std::fmt::Debug for Tag {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.as_ref().fmt(formatter)
	}
}

/// A reference to a tag.
///
/// Encapsulated in an enum to allow easier reference to name and category as applicable.
#[allow(single_use_lifetimes)] // otherwise PartialEq and Eq implementations cause warnings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ref<'a> {
	#[allow(missing_docs)]
	Name(&'a str),
	#[allow(missing_docs)]
	Category(&'a str),
	#[allow(missing_docs)]
	Both { category: &'a str, name: &'a str },
}

impl Ref<'_> {
	/// Convert back from [`Ref`] to [`Tag`] by cloning the data.
	///
	/// This is not provided as an implementation of `ToOwned` because there is a blanket impl of `ToOwned` for all implementors of `Clone`, which `Ref` implements.
	///
	/// This function will panic if it is a `Both` variant and the category is too long. However, this can only occur if the [`Ref`] was created from some other source than a [`Tag`], which is generally not a good idea.
	fn to_owned(self) -> Tag {
		match self {
			Self::Name(name) => Tag::name(name),
			Self::Category(category) => Tag::category(category),
			Self::Both { category, name } => Tag::both(category, name).unwrap(), /* We unwrap because we assume that the `Ref` came from a valid `Tag`. While this won't necessarily always be the case, it is the only use case we need to worry about. */
		}
	}
}

#[derive(Debug, Clone, Copy)]
enum TagKind {
	Name,
	Category,
	Both { name_start: u16 },
}

#[repr(C)]
struct TagInner {
	// this field has alignment of 2, so the whole struct does as well
	kind: TagKind,
	// this field has no alignment requirement, so there will be no padding before it
	data: str,
}

impl TagInner {
	fn new(data: &[&str], kind: TagKind) -> Box<Self> {
		use std::alloc::{alloc, handle_alloc_error, Layout};
		use std::ptr::{addr_of_mut, copy_nonoverlapping, slice_from_raw_parts_mut};

		let data_len = data.iter().map(|item| item.len()).sum::<usize>();
		let _ = isize::try_from(data_len).unwrap();

		let (layout, _) = Layout::new::<TagKind>()
			.extend(Layout::from_size_align(data_len, 1).unwrap())
			.unwrap();
		// make sure that the size is a multiple of the alignment
		let layout = layout.pad_to_align();
		// SAFETY: the layout will always have non-zero size
		let raw_allocation = unsafe { alloc(layout) };
		if raw_allocation.is_null() {
			handle_alloc_error(layout);
		}

		// we use the length *of the data* as the length of the slice pointer
		#[allow(clippy::cast_ptr_alignment)] // we know the alignment is correct
		let allocation = slice_from_raw_parts_mut(raw_allocation, data_len) as *mut TagInner;

		// SAFETY:
		// - the pointer is properly aligned since we used its alignment for the Layout above.
		// - the pointer is not null since we checked above.
		// - the memory range from using the pointer as TagKind is part of a single allocation, as the layout used for the allocation is at least the size of TagKind.
		// - `allocation` is dereferenceable as `TagInner` because it was allocated with the enough size and alignment and has been verified to not be null.
		unsafe {
			addr_of_mut!((*allocation).kind).write(kind);
		}

		let mut offset = 0;
		for datum in data {
			// SAFETY: since we allocated enough storage for all of the data, offsetting to the get the pointer to any datum within that data will be sound. we are offsetting in bytes so the offset can't overflow
			let this_datum_ptr = unsafe { addr_of_mut!((*allocation).data).cast::<u8>().offset(offset) };
			// SAFETY:
			// - since `src` came from `&str`, it is valid for reads of its bytes, and is properly aligned. we pass its length directly to the `count` parameter so we only read within the slice
			// - `dst` is valid for writes and is properly aligned because `str` has no alignment requirement so will be directly after the `kind` field, meaning that by allocating enough for `TagKind` plus `data.len()`, getting the address of the string component will result in a string pointer that points to an allocation that is valid for writes of `data.len()` bytes
			// - `dst` will not overlap `src` because `dst` is within our new allocation that we just created
			unsafe {
				copy_nonoverlapping(datum.as_ptr(), this_datum_ptr, datum.len());
			}
			// we were already able to convert the entire `data_len` to an `isize`, so unwrapping here shall not panic
			offset += isize::try_from(datum.len()).unwrap();
		}

		// SAFETY: `allocation` was allocated using the Global allocator, and the layout will be the same as the layout given by `for_value` on the resulting initialized reference
		unsafe { Box::from_raw(allocation) }
	}
}

/* OLD TAG CODE
/// A reference to a tag, category, or both.
/// Named `Tag` for simplicity
#[derive(Debug, PartialEq, Eq)]
pub enum Tag {
	/// A tag with the given name, in any category.
	/// Could refer to multiple tags
	Name(Box<str>),
	/// Any tag within the given category
	Category(Box<str>),
	/// The tag with the given name in the given category.
	/// Guaranteed to refer to at most one tag
	Both(Both),
}

/// The storage for a tag and category together.
/// Currently this stores them in a single string and stores the offset of the name
pub struct Both {
	// stores the start pointer and the full length. the `str` data is "<category><name>"
	storage: Box<str>,
	// indicates where within `storage` the name starts. the last byte of the category will thus be one byte before this
	name_start: usize,
}

impl Debug for Both {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("Both")
			.field("category", &self.category())
			.field("name", &self.name())
			.finish()
	}
}

impl Both {
	/// Create a new `Both` with the specified `category` and `name`
	pub fn new(category: &str, name: &str) -> Self {
		let mut storage = String::with_capacity(category.len() + name.len());
		storage += category;
		storage += name;
		debug_assert_eq!(storage.len(), storage.capacity());
		Self {
			storage: storage.into_boxed_str(),
			name_start: category.len(),
		}
	}

	/// Get the category
	pub fn category(&self) -> &str {
		&self.storage[0..self.name_start]
	}

	/// Get the name
	pub fn name(&self) -> &str {
		&self.storage[self.name_start..]
	}
}
*/
