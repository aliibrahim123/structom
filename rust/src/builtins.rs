use std::{collections::HashMap, sync::LazyLock};

pub static BUILT_INS_DEC_FILE: LazyLock<HashMap<&'static str, u16>> = LazyLock::new(|| {
	let map = HashMap::from([
		("any", 0x01),
		("bool", 0x08),
		("u8", 0x10),
		("i8", 0x11),
		("u16", 0x12),
		("i16", 0x13),
		("u32", 0x14),
		("i32", 0x15),
		("u64", 0x16),
		("i64", 0x17),
		("f16", 0x18),
		("f32", 0x19),
		("f64", 0x1a),
		("vuint", 0x1c),
		("vint", 0x1d),
		("buint", 0x1e),
		("bint", 0x1f),
		("str", 0x20),
		("array", 0x22),
		("map", 0x23),
		("inst", 0x30),
		("instN", 0x31),
		("dur", 0x32),
		("uuid", 0x33),
	]);

	map
});
