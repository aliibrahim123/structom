use alloc_crate::vec::Vec;
use thin_vec::ThinVec;

pub trait StrExt {
	fn char_at(&self, ind: usize) -> Option<char>;
}

impl StrExt for str {
	fn char_at(&self, ind: usize) -> Option<char> {
		self.get(ind..)?.chars().next()
	}
}

pub type HashMap<K, V> = hashbrown::HashMap<K, V, rustc_hash::FxBuildHasher>;

pub trait VecLike<T> {
	fn push(&mut self, val: T);
}
impl<T> VecLike<T> for Vec<T> {
	fn push(&mut self, val: T) {
		self.push(val)
	}
}
impl<T> VecLike<T> for ThinVec<T> {
	fn push(&mut self, val: T) {
		self.push(val)
	}
}
impl<T> VecLike<T> for () {
	fn push(&mut self, _val: T) {}
}

pub trait FromStrRadix: Sized {
	fn from_str_radix(src: &str, radix: u32) -> Result<Self, core::num::ParseIntError>;
}
macro_rules! impl_from_str_radix {
	[$($ty:ty),+] => {
		$(impl FromStrRadix for $ty {
			fn from_str_radix(src: &str, radix: u32) -> Result<Self, core::num::ParseIntError> {
				Self::from_str_radix(src, radix)
			}
		})+
	};
}
impl_from_str_radix![u8, u16, u32, u64, i8, i16, i32, i64];
