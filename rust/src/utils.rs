pub trait StrExt {
	fn char_at(&self, ind: usize) -> Option<char>;
}

impl StrExt for str {
	fn char_at(&self, ind: usize) -> Option<char> {
		self.get(ind..)?.chars().next()
	}
}

pub type HashMap<K, V> = hashbrown::HashMap<K, V, rustc_hash::FxBuildHasher>;
