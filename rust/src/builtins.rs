use std::{collections::HashMap, sync::LazyLock};

pub static BUILT_INS_IDS: LazyLock<HashMap<&'static str, u16>> = LazyLock::new(|| {
	let map = HashMap::from([
		("any", 0x01),
		("bool", 0x08),
		("u8", 0x10),
		("u16", 0x11),
		("u32", 0x12),
		("u64", 0x13),
		("i8", 0x14),
		("i16", 0x15),
		("i32", 0x16),
		("i64", 0x17),
		("f32", 0x18),
		("f64", 0x19),
		("vuint", 0x1c),
		("vint", 0x1d),
		("buint", 0x1e),
		("bint", 0x1f),
		("str", 0x20),
		("arr", 0x22),
		("map", 0x23),
		("inst", 0x30),
		("instN", 0x31),
		("dur", 0x32),
		("uuid", 0x33),
	]);

	map
});

pub static BUILT_INS_NAMES: LazyLock<HashMap<u16, &'static str>> = LazyLock::new(|| {
	let map = HashMap::from([
		(0x01, "any"),
		(0x08, "bool"),
		(0x10, "u8"),
		(0x11, "u16"),
		(0x12, "u32"),
		(0x13, "u64"),
		(0x14, "i8"),
		(0x15, "i16"),
		(0x16, "i32"),
		(0x17, "i64"),
		(0x18, "f32"),
		(0x19, "f64"),
		(0x1c, "vuint"),
		(0x1d, "vint"),
		(0x1e, "buint"),
		(0x1f, "bint"),
		(0x20, "str"),
		(0x22, "arr"),
		(0x23, "map"),
		(0x30, "inst"),
		(0x31, "instN"),
		(0x32, "dur"),
		(0x33, "uuid"),
	]);

	map
});

pub const ANY_TYPEID: u8 = 0x01;
pub const BOOL_TYPEID: u8 = 0x08;
pub const U8_TYPEID: u8 = 0x10;
pub const U16_TYPEID: u8 = 0x11;
pub const U32_TYPEID: u8 = 0x12;
pub const U64_TYPEID: u8 = 0x13;
pub const I8_TYPEID: u8 = 0x14;
pub const I16_TYPEID: u8 = 0x15;
pub const I32_TYPEID: u8 = 0x16;
pub const I64_TYPEID: u8 = 0x17;
pub const F32_TYPEID: u8 = 0x18;
pub const F64_TYPEID: u8 = 0x19;
pub const VUINT_TYPEID: u8 = 0x1c;
pub const VINT_TYPEID: u8 = 0x1d;
pub const BUINT_TYPEID: u8 = 0x1e;
pub const BINT_TYPEID: u8 = 0x1f;
pub const STR_TYPEID: u8 = 0x20;
pub const ARR_TYPEID: u8 = 0x22;
pub const MAP_TYPEID: u8 = 0x23;
pub const INST_TYPEID: u8 = 0x30;
pub const INSTN_TYPEID: u8 = 0x31;
pub const DUR_TYPEID: u8 = 0x32;
pub const UUID_TYPEID: u8 = 0x33;
