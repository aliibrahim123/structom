export interface Buffer {
	buf: Uint8Array,
	view: DataView
	pos: number
}

export interface Cursor {
	pos: number
}

export function reserve(buf: Buffer, add_size: number) {
	let old_size = buf.buf.length 
	if (buf.pos + add_size > old_size) {
		let new_size = old_size > 2 ** 20 ? old_size + 2 ** 10 : old_size * 2;
		if (new_size < buf.pos + add_size) new_size = buf.pos + add_size
		let new_buf = new Uint8Array(buf.buf.length * 2)
		new_buf.set(buf.buf)
		buf.buf = new_buf;
		buf.view = new DataView(new_buf.buffer)
	}
}

export function encode_u8(buf: Buffer, value: number) {
	reserve(buf, 1);
	buf.buf[buf.pos] = value;
	buf.pos += 1;
}
export function encode_u16(buf: Buffer, value: number) {
	reserve(buf, 2);
	buf.view.setUint16(buf.pos, value, true);
	buf.pos += 2;
}
export function encode_u32(buf: Buffer, value: number) {
	reserve(buf, 4);
	buf.view.setUint32(buf.pos, value, true);
	buf.pos += 4;
}
export function encode_u64(buf: Buffer, value: bigint) {
	reserve(buf, 8);
	buf.view.setBigUint64(buf.pos, value, true);
	buf.pos += 8;
}

export function encode_i8(buf: Buffer, value: number) {
	reserve(buf, 1);
	buf.view.setInt8(buf.pos, value);
	buf.pos += 1;
}
export function encode_i16(buf: Buffer, value: number) {
	reserve(buf, 2);
	buf.view.setInt16(buf.pos, value, true);
	buf.pos += 2;
}
export function encode_i32(buf: Buffer, value: number) {
	reserve(buf, 4);
	buf.view.setInt32(buf.pos, value, true);
	buf.pos += 4;
}
export function encode_i64(buf: Buffer, value: bigint) {
	reserve(buf, 8);
	buf.view.setBigInt64(buf.pos, value, true);
	buf.pos += 8;
}

export function decode_u8(buf: Buffer, cur: Cursor) {
	let res = buf.buf[cur.pos];
	cur.pos += 1;
	return res;
}
export function decode_u16(buf: Buffer, cur: Cursor) {
	let res = buf.view.getUint16(cur.pos, true);
	cur.pos += 2;
	return res;
}
export function decode_u32(buf: Buffer, cur: Cursor) {
	let res = buf.view.getUint32(cur.pos, true);
	cur.pos += 4;
	return res;
}
export function decode_u64(buf: Buffer, cur: Cursor) {
	let res = buf.view.getBigUint64(cur.pos, true);
	cur.pos += 8;
	return res;
}

export function decode_i8(buf: Buffer, cur: Cursor) {
	let res = buf.view.getInt8(cur.pos);
	cur.pos += 1;
	return res;
}
export function decode_i16(buf: Buffer, cur: Cursor) {
	let res = buf.view.getInt16(cur.pos, true);
	cur.pos += 2;
	return res;
}
export function decode_i32(buf: Buffer, cur: Cursor) {
	let res = buf.view.getInt32(cur.pos, true);
	cur.pos += 4;
	return res;
}
export function decode_i64(buf: Buffer, cur: Cursor) {
	let res = buf.view.getBigInt64(cur.pos, true);
	cur.pos += 8;
	return res;
}

export function encode_u8_arr(buf: Buffer, value: ArrayLike<number>) {
	reserve(buf, value.length);
	buf.buf.set(value, buf.pos);
	buf.pos += value.length;
}
export function decode_u8_arr(buf: Buffer, size: number, cur: Cursor) {
	let res = buf.buf.slice(cur.pos, cur.pos + size);
	cur.pos += size;
	return res
}