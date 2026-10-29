use lean_string::LeanStr;

use crate::decl::{
	ConstructError,
	id::ItemStore,
	items::{Body, VariantTag},
	rich::{Metadata, RichNotation},
};

#[derive(Debug)]
pub struct Variant {
	name: LeanStr,
	tag: VariantTag,
	body: Option<Body>,
	rich_notation: RichNotation,
	metadata: Metadata,
}

impl Variant {
	pub fn new(name: &str, body: Option<Body>) -> Self {
		Self::new_lean(LeanStr::from(name), body)
	}
	pub(crate) fn new_lean(name: LeanStr, body: Option<Body>) -> Self {
		Self {
			name,
			tag: VariantTag::UNASSIGNED,
			body,
			rich_notation: RichNotation::new(),
			metadata: Metadata::new(),
		}
	}
	pub fn name(&self) -> &str {
		&self.name
	}
	pub fn tag(&self) -> VariantTag {
		self.tag
	}
	pub fn body(&self) -> Option<&Body> {
		self.body.as_ref()
	}
	pub fn is_reserved(&self) -> bool {
		matches!(self.name.as_str(), "_")
	}
	pub fn rich_notation(&self) -> &RichNotation {
		&self.rich_notation
	}
	pub fn rich_notation_mut(&mut self) -> &mut RichNotation {
		&mut self.rich_notation
	}
	pub fn metadata(&self) -> &Metadata {
		&self.metadata
	}
	pub fn metadata_mut(&mut self) -> &mut Metadata {
		&mut self.metadata
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagType {
	U8,
	U16,
	U32,
	Auto,
}
impl TagType {
	fn max(&self) -> u32 {
		match self {
			Self::U8 => u8::MAX as u32,
			Self::U16 => u16::MAX as u32,
			Self::U32 => u32::MAX,
			Self::Auto => u32::MAX,
		}
	}
}

#[derive(Debug)]
pub struct Union {
	tag_type: TagType,
	variants: ItemStore<Variant, { u32::MAX }, true>,
}
impl Default for Union {
	fn default() -> Self {
		Self::new(TagType::Auto)
	}
}
impl Union {
	pub fn new(tag_type: TagType) -> Self {
		Self { tag_type, variants: ItemStore::new() }
	}
	pub fn add_variant(
		&mut self, variant: Variant,
	) -> Result<VariantTag, ConstructError> {
		if self.variants.id_counter >= self.tag_type.max() {
			return Err(ConstructError::FullCapacity);
		}
		self.variants.add(variant.name.clone(), variant).map(|(var, id)| {
			var.tag = VariantTag(id);
			var.tag
		})
	}
	pub fn add_variant_at(
		&mut self, variant: Variant, tag: usize,
	) -> Result<VariantTag, ConstructError> {
		if tag as u32 > self.tag_type.max() {
			return Err(ConstructError::FullCapacity);
		}
		self.variants.add_at(variant.name.clone(), variant, tag).map(|(var, id)| {
			var.tag = VariantTag(id);
			var.tag
		})
	}
	pub fn reserve_variant(
		&mut self, id: Option<usize>, body: Option<Body>,
	) -> Result<VariantTag, ConstructError> {
		let variant = Variant::new("_", body);
		match id {
			Some(id) => self.add_variant_at(variant, id),
			None => self.add_variant(variant),
		}
	}
	pub fn tag_type(&self) -> TagType {
		match (self.tag_type, self.variants.id_counter) {
			(TagType::Auto, 0..=255) => TagType::U8,
			(TagType::Auto, 256..=65535) => TagType::U16,
			(TagType::Auto, _) => TagType::U32,
			_ => self.tag_type,
		}
	}
	pub fn variant_by_name(&self, name: &str) -> Option<&Variant> {
		self.variants.get_by_name(name)
	}
	pub fn variant_by_tag(&self, tag: VariantTag) -> Option<&Variant> {
		self.variants.get_by_id(tag.0)
	}
	pub fn variant_count(&self) -> usize {
		self.variants.items.len()
	}
	pub fn variants(&self) -> &[Variant] {
		&self.variants.items
	}
}
