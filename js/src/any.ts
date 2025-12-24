import { decode_i16, decode_i32, decode_i64, decode_i8, decode_u16, decode_u32, decode_u64, decode_u8, type Buffer, type Cursor } from "./buf.ts";
import { decode_bool, decode_str, decode_arr, decode_map } from "./general.ts";
import type { Value } from "./index.ts";
import { decode_f32, decode_f64, decode_vint, decode_vuint } from "./number.ts";
import { decode_dur, decode_inst, decode_instN, decode_uuid } from "./rich.ts";

const any_typeid = 0x01;
const bool_typeid = 0x08;
const u8_typeid = 0x10;
const u16_typeid = 0x11;
const u32_typeid = 0x12;
const u64_typeid = 0x13;
const i8_typeid = 0x14;
const i16_typeid = 0x15;
const i32_typeid = 0x16;
const i64_typeid = 0x17;
const f32_typeid = 0x18;
const f64_typeid = 0x19;
const vuint_typeid = 0x1c;
const vint_typeid = 0x1d;
const str_typeid = 0x20;
const arr_typeid = 0x22;
const map_typeid = 0x23;
const inst_typeid = 0x30;
const instn_typeid = 0x31;
const dur_typeid = 0x32;
const uuid_typeid = 0x33;

function decode_value(buf: Buffer, typeid: number, cur: Cursor): Value {
	switch (typeid) {
		case any_typeid: return decode_any(buf, cur); 
		case bool_typeid: return decode_bool(buf, cur);

		case u8_typeid:  return decode_u8(buf, cur);
		case u16_typeid: return decode_u16(buf, cur);
		case u32_typeid: return decode_u32(buf, cur);
		case u64_typeid: return decode_u64(buf, cur);

		case i8_typeid:  return decode_i8(buf, cur);
		case i16_typeid: return decode_i16(buf, cur);
		case i32_typeid: return decode_i32(buf, cur);
		case i64_typeid: return decode_i64(buf, cur);

		case f32_typeid: return decode_f32(buf, cur);
		case f64_typeid: return decode_f64(buf, cur);

		case vuint_typeid: return decode_vuint(buf, cur);
		case vint_typeid: return decode_vint(buf, cur);

		case str_typeid: return decode_str(buf, cur);
		case inst_typeid: return decode_inst(buf, cur);
		case instn_typeid: return decode_instN(buf, cur);
		case dur_typeid: return decode_dur(buf, cur);
		case uuid_typeid: return decode_uuid(buf, cur);

		case arr_typeid: {
			let item_id = decode_u8(buf, cur);	
			return decode_arr(buf, cur, (buf, cur) => decode_value(buf, item_id, cur))
		}
		case map_typeid: {
			let key_id = decode_u8(buf, cur);
			let value_id = decode_u8(buf, cur);
			return decode_map(buf, cur, 
				(buf, cur) => decode_value(buf, key_id, cur), 
				(buf, cur) => decode_value(buf, value_id, cur)
			)
		}
	}
	return undefined as any
}

export function decode_any (buf: Buffer, cur: Cursor): Value {
	return decode_value(buf, decode_u8(buf, cur), cur);
}

