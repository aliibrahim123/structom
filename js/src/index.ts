import { decode_any, encode_any } from "./any.ts";
import { decode_u8, encode_u8, type Buffer, type Cursor } from "./buf.ts";

export interface UUID {
	type: 'uuid',
	value: Uint8Array
};
export interface Dur {
	type: 'dur',
	value: bigint
}
export type Value = 
	boolean | number | string | bigint | Date | UUID | Dur | Array<Value> | Map<Value, Value>;

export function encode(value: Value) {
	let _buf = new Uint8Array(256);
	let buf: Buffer = { buf: _buf, pos: 0, view: new DataView(_buf.buffer) };
	encode_u8(buf, 0);
	encode_any(buf, value);
	return buf.buf.slice(0, buf.pos);
}

export function decode(data: ArrayBuffer) {
	let buf: Buffer = { buf: new Uint8Array(data), pos: 0, view: new DataView(data) };
	let cur: Cursor = { pos: 0 };
	if (decode_u8(buf, cur) !== 0) return;
	let value = decode_any(buf, cur);
	if (cur.pos !== data.byteLength) return;
	return value;
}

export * from './any.ts';
export * from './general.ts';
export * from './number.ts';
export * from './rich.ts';
export { 
	decode_i8, decode_i16, decode_i32, decode_i64, decode_u8, decode_u16, decode_u32, decode_u64, 
	decode_u8_arr, encode_i8, encode_i16, encode_i32, encode_i64, encode_u8, encode_u16, encode_u32, 
	encode_u64, encode_u8_arr 
} from './buf.ts';