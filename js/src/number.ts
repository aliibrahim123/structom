import { decode_u8, encode_u8, reserve, type Buffer, type Cursor } from "./buf.ts";

export function encode_f32 (buf: Buffer, value: number) {
	reserve(buf, 4);
	buf.view.setFloat32(buf.pos, value, true);
	buf.pos += 4;
}
export function encode_f64 (buf: Buffer, value: number) {
	reserve(buf, 8);
	buf.view.setFloat64(buf.pos, value, true);
	buf.pos += 8;
}

export function decode_f32 (buf: Buffer, cur: Cursor) {
	let res = buf.view.getFloat32(cur.pos, true);
	cur.pos += 4;
	return res
}
export function decode_f64 (buf: Buffer, cur: Cursor) {
	let res = buf.view.getFloat64(cur.pos, true);
	cur.pos += 8;
	return res
}

export function encode_vuint(buf: Buffer, value: number | bigint) {
	let cond = true;

	if (typeof(value) === 'number' && Math.abs(value) < 2 ** 31) {
		// while there is input
		while (cond) {
			// extract least significant 7 bits
			let byte = value & 0b0111_1111;
			// shift to next section
			value >>= 7;
			// add continuation bit (0 = end byte)
			encode_u8(buf, value == 0 ? byte : byte | 0b1000_0000 );

			cond = value != 0;
		}
	} else {
		value = BigInt(value);
		// while there is input
		while (cond) {
			// extract least significant 7 bits
			let byte = Number(value & 0b0111_1111n);
			// shift to next section
			value >>= 7n;
			// add continuation bit (0 = end byte)
			encode_u8(buf, value == 0n ? byte : byte | 0b1000_0000 );

			cond = value != 0n;
		}
	}
}
export function encode_vint(buf: Buffer, value: number | bigint) {
	let cond = true;

	if (typeof(value) === 'number' && Math.abs(value) < 2 ** 31) {
		// while there is input
		while (cond) {
			// extract least significant 7 bits
			let byte = value & 0b0111_1111;
			// shift to next section
			value >>= 7;
			// ensure at least 1 sign bit is encoded (0 for positive and 1 for negative)
			let sign_bit = byte & 0b0100_0000;
			if ((value == 0 && sign_bit == 0) || (value == -1 && sign_bit != 0))
				cond = false;
			// add continuation bit (0 = end byte)
			else byte |= 0b1000_0000;
			// add continuation bit (0 = end byte)
			encode_u8(buf, byte);
		}
	} else {
		value = BigInt(value);
		// while there is input
		while (cond) {
			// extract least significant 7 bits
			let byte = Number(value & 0b0111_1111n);
			// shift to next section
			value >>= 7n;
			// ensure at least 1 sign bit is encoded (0 for positive and 1 for negative)
			let sign_bit = byte & 0b0100_0000;
			if ((value == 0n && sign_bit == 0) || (value == -1n && sign_bit != 0))
				cond = false;
			// add continuation bit (0 = end byte)
			else byte |= 0b1000_0000;
			// add continuation bit (0 = end byte)
			encode_u8(buf, byte);
		}
	}
}

export function decode_vuint (buf: Buffer, cur: Cursor) {
	let res = 0n, shift = 0n, cond = true;
	
	// while there is input
	while (cond) {
		let byte = decode_u8(buf, cur);
		// add the least significant 7 bits to the next section of the result
		res |= BigInt(byte & 0b0111_1111) << shift;
		// next section
		shift += 7n;
		// if the continuation bit is set, continue
		cond = (byte & 0b1000_0000) != 0;
	}

	return res > 2 ** 50 ? res : Number(res);
}
export function decode_vint(buf: Buffer, cur: Cursor) {
	let res = 0n, shift = 0n, cond = true, byte = 0;
	
	// while there is input
	while (cond) {
		byte =  decode_u8(buf, cur);
		// add the least significant 7 bits to the next section of the result
		res |= BigInt(byte & 0b0111_1111) << shift;
		// next section
		shift += 7n;
		// if the continuation bit is set, continue
		cond = (byte & 0b1000_0000) != 0;
	}

	// if the value is neg, sign extend it
	if ((shift < 64n) && ((byte & 0b0100_0000) !== 0)) {
		res |= ~0n << shift;
	}

	return (res > 0 ? res : -res) > 2 ** 50 ? res : Number(res);
}

export function encode_vuint_pre_aloc(
	buf: Buffer, value: number, start_ind: number, pre_aloc: number
) {
	let nb_buf = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
	let size = 0, cond = true;

	while (cond) {
		// extract least significant 7 bits
		let byte = value & 0b0111_1111;
		// shift to next section
		value >>= 7;
		// add continuation bit (0 = end byte)
		nb_buf[size] = value == 0 ? byte : byte | 0b1000_0000 ;

		cond = value != 0;
		size += 1;
	}

	// case size is larger than pre allocated space, expand to fit
	if (size > pre_aloc) {
		let len = buf.pos;
		reserve(buf, size - pre_aloc);
		buf.buf.copyWithin(start_ind + size, start_ind + pre_aloc, len);
	}

	// set continuation bits in all pre allocated area, even if not used, making it padding
	if (size < pre_aloc) {
		for (let i = 0; i < pre_aloc - 1; i++) {
			nb_buf[i] |= 0b1000_0000
		}
		size = pre_aloc;
	}

	for (let ind = 0; ind < size; ind++) buf.buf[ind + start_ind] = nb_buf[ind];
}