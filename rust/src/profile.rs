use core::{
	fmt::{Debug, Display, Write},
	ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not},
};

macro_rules! bit_markers {
	($first:ident $(, $rest:ident)* $(,)?) => {
    	pub const $first: u16 = 1;
    	bit_markers!(#next $first; $($rest),*);
    };
    (#next $prev:ident; $cur:ident $(, $rest:ident)*) => {
    	pub const $cur: u16 = $prev << 1;
    	bit_markers!(#next $cur; $($rest),*);
    };
    (#next $prev:ident;) => {};
}

mod fields {
	bit_markers![
		OBJ,
		BIN,
		BIN_SEG,
		DECL,
		DECL_GEN,
		RICH,
		RICH_UUID,
		RICH_INST,
		RICH_DUR,
		RICH_BINT,
		RICH_DEC,
		RICH_COLOR,
		RICH_BITFIELD,
		RICH_UNITFIELD
	];
	pub const OBJ_GRP: u16 = OBJ;
	pub const BIN_GRP: u16 = BIN | BIN_SEG;
	pub const DECL_GRP: u16 = DECL | DECL_GEN;
	pub const RICH_GRP: u16 = RICH
		| RICH_UUID
		| RICH_INST
		| RICH_DUR
		| RICH_BINT
		| RICH_DEC
		| RICH_COLOR
		| RICH_BITFIELD
		| RICH_UNITFIELD;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Profile(u16);

impl Profile {
	pub fn new() -> Self {
		Self::default()
	}

	pub const fn supported() -> Self {
		let value = if cfg!(feature = "parse") { fields::OBJ } else { 0 }
			| if cfg!(feature = "bin") { fields::BIN | fields::BIN_SEG } else { 0 }
			| (fields::DECL | fields::DECL_GEN | fields::RICH)
			| if cfg!(feature = "rich-uuid") { fields::RICH_UUID } else { 0 }
			| if cfg!(feature = "rich-inst") { fields::RICH_INST } else { 0 }
			| if cfg!(feature = "rich-dur") { fields::RICH_DUR } else { 0 }
			| if cfg!(feature = "rich-bint") { fields::RICH_BINT } else { 0 }
			| if cfg!(feature = "rich-dec") { fields::RICH_DEC } else { 0 }
			| if cfg!(feature = "rich-color") { fields::RICH_COLOR } else { 0 }
			| if cfg!(feature = "rich-bitfield") { fields::RICH_BITFIELD } else { 0 }
			| if cfg!(feature = "rich-unitfield") { fields::RICH_UNITFIELD } else { 0 };
		Self(value)
	}
	pub fn supports(self, required: Profile) -> bool {
		self.0 & required.0 == required.0
	}

	pub fn obj(self) -> Option<Obj> {
		(self.0 & fields::OBJ != 0).then_some(Obj(self.0 & fields::OBJ_GRP))
	}
	pub fn bin(self) -> Option<Bin> {
		(self.0 & fields::BIN != 0).then_some(Bin(self.0 & fields::BIN_GRP))
	}
	pub fn decl(self) -> Option<Decl> {
		(self.0 & fields::DECL != 0).then_some(Decl(self.0 & fields::DECL_GRP))
	}
	pub fn rich(self) -> Option<Rich> {
		(self.0 & fields::RICH != 0).then_some(Rich(self.0 & fields::RICH_GRP))
	}

	pub fn with_obj(mut self, obj: Option<Obj>) -> Self {
		self.0 = (self.0 & !fields::OBJ_GRP) | obj.map_or(0, |o| o.0);
		self
	}
	pub fn with_bin(mut self, bin: Option<Bin>) -> Self {
		self.0 = (self.0 & !fields::BIN_GRP) | bin.map_or(0, |b| b.0);
		self
	}
	pub fn with_decl(mut self, decl: Option<Decl>) -> Self {
		self.0 = (self.0 & !fields::DECL_GRP) | decl.map_or(0, |d| d.0);
		self
	}
	pub fn with_rich(mut self, rich: Option<Rich>) -> Self {
		self.0 = (self.0 & !fields::RICH_GRP) | rich.map_or(0, |r| r.0);
		self
	}
}

impl Display for Profile {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let mut is_first = true;
		write_group(f, self.obj(), &mut is_first)?;
		write_group(f, self.bin(), &mut is_first)?;
		write_group(f, self.decl(), &mut is_first)?;
		write_group(f, self.rich(), &mut is_first)?;
		Ok(())
	}
}
fn write_group(
	f: &mut core::fmt::Formatter<'_>, group: Option<impl Display>, is_first: &mut bool,
) -> core::fmt::Result {
	if let Some(group) = group {
		if !*is_first {
			f.write_char('_')?;
		}
		Display::fmt(&group, f)?;
		*is_first = false;
	}
	Ok(())
}

impl BitOr for Profile {
	type Output = Self;
	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}
impl BitOrAssign for Profile {
	fn bitor_assign(&mut self, rhs: Self) {
		self.0 |= rhs.0;
	}
}
impl BitAnd for Profile {
	type Output = Self;
	fn bitand(self, rhs: Self) -> Self::Output {
		Self(self.0 & rhs.0)
	}
}
impl BitAndAssign for Profile {
	fn bitand_assign(&mut self, rhs: Self) {
		self.0 &= rhs.0;
	}
}

macro_rules! group {
	(struct $name:ident, $display_name:literal, $flag_field:ident, $group_field:ident,
		[$(($feat:ident, $feat_display:literal, $feat_flag:ident, $feat_const:ident)),*]
	) => {
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
		pub struct $name(u16);
		impl $name {
			pub fn new() -> Self {
				Self::default()
			}

			$(pub const $feat_const: Self = Self(fields::$feat_flag | fields::$flag_field);)*
			$(pub fn $feat(self) -> bool {
				self.0 & fields::$feat_flag != 0
			})*
		}
		impl Default for $name {
			fn default() -> Self {
				Self(fields::$flag_field)
			}
		}
		impl Not for $name {
			type Output = Self;
			fn not(self) -> Self::Output {
				Self(!self.0 & fields::$group_field | fields::$flag_field)
			}
		}
		impl BitOr for $name {
			type Output = Self;
			fn bitor(self, rhs: Self) -> Self::Output {
				Self(self.0 | rhs.0)
			}
		}
		impl BitOrAssign for $name {
			fn bitor_assign(&mut self, rhs: $name) {
				self.0 |= rhs.0
			}
		}
		impl BitOr<$name> for Profile {
			type Output = Profile;
			fn bitor(self, rhs: $name) -> Profile { Profile(self.0 | rhs.0) }
		}
		impl BitOrAssign<$name> for Profile {
			fn bitor_assign(&mut self, rhs: $name) {
				self.0 |= rhs.0
			}
		}
		impl BitAnd for $name {
			type Output = Self;
			fn bitand(self, rhs: Self) -> Self::Output {
				Self(self.0 & rhs.0)
			}
		}
		impl From<$name> for Profile {
			fn from(value: $name) -> Self {
				Self(value.0)
			}
		}
		impl Display for $name {
			#[allow(unused)]
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				f.write_str($display_name)?;
				if self.0 != fields::$flag_field {
					f.write_char('{')?;
					let mut first = true;
					$(write_feat(f, $feat_display, self.$feat(), &mut first)?;)*
					f.write_char('}')?;
				}
				Ok(())
			}
		}
	};
}
fn write_feat(
	f: &mut core::fmt::Formatter<'_>, feat: &str, enabled: bool, is_first: &mut bool,
) -> core::fmt::Result {
	if enabled {
		if !*is_first {
			f.write_char(',')?;
		}
		f.write_str(feat)?;
		*is_first = false;
	}
	Ok(())
}

group!(struct Obj, "obj", OBJ, OBJ_GRP, []);

group!(struct Bin, "bin", BIN, BIN_GRP, [(segments, "seg", BIN_SEG, SEG)]);

group!(struct Decl, "decl", DECL, DECL_GRP, [(generics, "gen", DECL_GEN, GEN)]);

group!(struct Rich, "rich", RICH, RICH_GRP, [
	(uuid, "uuid", RICH_UUID, UUID),
	(inst, "inst", RICH_INST, INST),
	(dur, "dur", RICH_DUR, DUR),
	(bint, "bint", RICH_BINT, BINT),
	(dec, "dec", RICH_DEC, DEC),
	(color, "color", RICH_COLOR, COLOR),
	(bitfield, "bitfield", RICH_BITFIELD, BITFIELD),
	(unitfield, "unitfield", RICH_UNITFIELD, UNITFIELD)
]);

#[cfg(feature = "parse")]
mod parse {
	use core::str::FromStr;

	use alloc_crate::vec::Vec;

	use crate::parse::{
		ParseError, head::parse_profile, str_cursor::Cursor, tokens::Source,
	};

	use super::*;
	impl FromStr for Profile {
		type Err = Vec<ParseError>;
		fn from_str(s: &str) -> Result<Self, Self::Err> {
			let mut errors = Vec::new();
			let source = Source::new(s, "input");
			let mut cur = Cursor::new(&source, None, &mut errors);
			let profile = parse_profile(&mut cur, None, false).0;
			match errors.is_empty() {
				true => Ok(profile),
				false => Err(errors),
			}
		}
	}
}
