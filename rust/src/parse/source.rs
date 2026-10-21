use core::cell::OnceCell;

use alloc_crate::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source<'a> {
	pub str: &'a str,
	pub path: &'a str,
	pub line_poses: OnceCell<Vec<usize>>,
}
impl Source<'_> {
	pub fn new<'a>(str: &'a str, path: &'a str) -> Source<'a> {
		Source { str, path, line_poses: OnceCell::new() }
	}
	pub fn source_of(&self, span: Span) -> &str {
		&self.str[span.start()..span.end()]
	}
	fn init_line_poses(&self) -> Vec<usize> {
		self.str.match_indices('\n').map(|(ind, _)| ind).collect()
	}
	pub fn pos_of(&self, ind: usize) -> (usize, usize) {
		let Source { str: src, line_poses, .. } = self;
		let line_poses = line_poses.get_or_init(|| self.init_line_poses());
		let nl_before = line_poses.partition_point(|&p| p < ind);
		if nl_before == 0 {
			(1, src[..ind].chars().count() + 1)
		} else {
			(nl_before + 1, src[line_poses[nl_before - 1]..ind].chars().count())
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Span {
	#[cfg(not(feature = "large-span"))]
	pub(crate) start: u32,
	#[cfg(not(feature = "large-span"))]
	pub(crate) end: u32,
	#[cfg(feature = "large-span")]
	pub(crate) start: u64,
	#[cfg(feature = "large-span")]
	pub(crate) end: u64,
}

impl Span {
	pub const EMPTY: Span = Span { start: 0, end: 0 };
	pub(crate) fn new(start: usize, end: usize) -> Self {
		Self { start: start as _, end: end as _ }
	}
	pub fn start(&self) -> usize {
		self.start as usize
	}
	pub fn end(&self) -> usize {
		self.end as usize
	}
	pub fn is_point(&self) -> bool {
		self.start + 1 == self.end
	}
	pub fn point(pos: usize) -> Self {
		Self::new(pos, pos + 1)
	}
}
