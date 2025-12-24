import { decode_u8, decode_u8_arr, encode_u8, encode_u8_arr, reserve, type Buffer, type Cursor } from "./buf.ts";
import { decode_vuint, encode_vuint } from "./number.ts";

export function encode_bool(buf: Buffer, value: boolean) {
	encode_u8(buf, value === true ? 1 : 0)
}
export function decode_bool(buf: Buffer, cur: Cursor) {
	return decode_u8(buf, cur) !== 0
}

let TextEnc = new TextEncoder(); 
export function encode_str(buf: Buffer, value: string) {
	let res = TextEnc.encode(value);
	encode_vuint(buf, res.length);
	encode_u8_arr(buf, res);
}

let TextDec = new TextDecoder('utf-8');
export function decode_str (buf: Buffer, cur: Cursor) {
	let size = decode_vuint(buf, cur) as number;
	return TextDec.decode(decode_u8_arr(buf, size, cur));
}

type Encoder <T> = (buf: Buffer, value: T) => void;
type Decoder <T> = (buf: Buffer, cur: Cursor) => T;

export function encode_arr<T> (buf: Buffer, value: T[], item_fn: Encoder<T>, in_field = false) {
	if (!in_field) 
		encode_vuint(buf, value.length);
	for (let val of value) 
		item_fn(buf, val);
}
export function decode_arr<T> (buf: Buffer, cur: Cursor, item_fn: Decoder<T>, in_field = false) {
	let len = decode_vuint(buf, cur) as number;
	if (in_field) {
		let start_ind = cur.pos;
		let arr: T[] = [];
		while (cur.pos < start_ind + len)
			arr.push(item_fn(buf, cur));
		return arr
	} else {
		let arr: T[] = [];
		for (let i = 0; i < len; i++) 
			arr.push(item_fn(buf, cur));
		return arr
	}
}

export function encode_map<K, V> (
	buf: Buffer, value: Map<K, V>, key_fn: Encoder<K>, val_fn: Encoder<V>, 
	in_field = false
) {
	if (!in_field) 
		encode_vuint(buf, value.size);
	for (let [k, v] of value) {
		key_fn(buf, k);
		val_fn(buf, v);
	}
}
export function decode_map<K, V> (
	buf: Buffer, cur: Cursor, key_fn: Decoder<K>, val_fn: Decoder<V>, 
	in_field = false
) {
	let len = decode_vuint(buf, cur) as number;
	if (in_field) {
		let start_ind = cur.pos;
		let map = new Map<K, V>();
		while (cur.pos < start_ind + len) 
			map.set(key_fn(buf, cur), val_fn(buf, cur));
		return map
	} else {
		let map = new Map<K, V>();
		for (let i = 0; i < len; i++) {
			map.set(key_fn(buf, cur), val_fn(buf, cur));}
		return map
	}
}

export function skip_field (buf: Buffer, cur: Cursor, header: number) {
	switch (header & 0b111) {
		case 0b000: cur.pos += 1; break;
		case 0b001: cur.pos += 2; break;
		case 0b010: cur.pos += 4; break;
		case 0b011: cur.pos += 8; break;
		// decode vuint and ignore
		case 0b100: decode_vuint(buf, cur); break;
		// len field is encoded
		case 0b101: cur.pos += decode_vuint(buf, cur) as number;
	}
}