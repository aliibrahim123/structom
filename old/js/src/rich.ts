import { decode_i64, decode_u32, decode_u8_arr, encode_i64, encode_u32, encode_u8_arr, type Buffer, type Cursor } from "./buf.ts";
import type { Dur, UUID } from "./index.ts";

export function encode_uuid (buf: Buffer, value: UUID) {
	encode_u8_arr(buf, value.value);
}
export function decode_uuid (buf: Buffer, cur: Cursor) {
	return { type: 'uuid', value: decode_u8_arr(buf, 16, cur) } satisfies UUID
}

export function encode_inst(buf: Buffer, value: Date) {
	encode_i64(buf, BigInt(Number(value)))
}
export function encode_instN(buf: Buffer, value: Date) {
	encode_i64(buf, BigInt(Number(value)))
	encode_u32(buf, 0)
}
export function decode_inst(buf: Buffer, cur: Cursor) {
	return new Date(Number(decode_i64(buf, cur)))
}
export function decode_instN(buf: Buffer, cur: Cursor) {
	let res = decode_i64(buf, cur);
	decode_u32(buf, cur);
	return new Date(Number(res))
}

export function encode_dur(buf: Buffer, value: Dur) {
	encode_i64(buf, value.value)
}
export function decode_dur(buf: Buffer, cur: Cursor) {
	return { type: 'dur', value: decode_i64(buf, cur) } satisfies Dur
}