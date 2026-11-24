use lean_string::LeanStr;

use crate::decl::{
	AssignError,
	id::ItemStore,
	items::{Body, VariantTag},
	rich::{Metadata, RichNotation},
};

#[derive(Debug, Clone)]
pub struct Variant {
	name: LeanStr,
	tag: VariantTag,
	pub(crate) body: Option<Body>,
	pub(crate) rich_notation: RichNotation,
	pub(crate) metadata: Metadata,
}

impl Variant {
	pub fn new(
		name: &str, body: Option<Body>,
	) -> Result<Self, (Option<Body>, AssignError)> {
		Self::new_lean(LeanStr::from(name), body)
	}
	pub(crate) fn new_lean(
		name: LeanStr, mut body: Option<Body>,
	) -> Result<Self, (Option<Body>, AssignError)> {
		if let Some(Body::Fields(fields)) = &mut body
			&& fields.finalize().is_err()
		{
			return Err((body, AssignError::UnassignedId));
		}
		Ok(Self {
			name,
			tag: VariantTag::UNASSIGNED,
			body,
			rich_notation: RichNotation::new(),
			metadata: Metadata::new(),
		})
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
	pub fn max(&self) -> u32 {
		match self {
			Self::U8 => u8::MAX as u32,
			Self::U16 => u16::MAX as u32,
			Self::U32 => u32::MAX,
			Self::Auto => u32::MAX,
		}
	}
}

#[derive(Debug, Clone)]
pub struct Union {
	tag_type: TagType,
	pub(crate) variants: ItemStore<Variant, { u32::MAX }, true>,
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
	) -> Result<VariantTag, (Variant, AssignError)> {
		if self.variants.id_counter >= self.tag_type.max() {
			return Err((variant, AssignError::FullCapacity));
		}
		self.variants.add(variant.name.clone(), variant).map(|(var, id)| {
			var.tag = VariantTag(id);
			var.tag
		})
	}
	pub fn add_variant_at(
		&mut self, variant: Variant, tag: u32,
	) -> Result<VariantTag, (Variant, AssignError)> {
		if tag > self.tag_type.max() {
			return Err((variant, AssignError::IdOutOfRange));
		}
		self.variants.add_at(variant.name.clone(), variant, tag as _).map(|(var, id)| {
			var.tag = VariantTag(id);
			var.tag
		})
	}
	pub fn reserve_variant(
		&mut self, id: Option<u32>, body: Option<Body>,
	) -> Result<VariantTag, (Option<Body>, AssignError)> {
		let variant = Variant::new("_", body)?;
		match id {
			Some(id) => self.add_variant_at(variant, id),
			None => self.add_variant(variant),
		}
		.map_err(|(var, err)| (var.body, err))
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
