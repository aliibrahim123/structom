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
impl VecLike<()> for () {
	fn push(&mut self, _val: ()) {}
}
