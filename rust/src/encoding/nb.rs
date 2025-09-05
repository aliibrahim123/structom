#[inline]
pub fn encode_u8(data: &mut Vec<u8>, value: u8) {
	data.push(value);
}
#[inline]
pub fn encode_u16(data: &mut Vec<u8>, value: u16) {
	data.extend_from_slice(&value.to_le_bytes());
}
#[inline]
pub fn encode_u32(data: &mut Vec<u8>, value: u32) {
	data.extend_from_slice(&value.to_le_bytes());
}
#[inline]
pub fn encode_u64(data: &mut Vec<u8>, value: u64) {
	data.extend_from_slice(&value.to_le_bytes());
}

#[inline]
pub fn encode_i8(data: &mut Vec<u8>, value: i8) {
	data.push(value.to_le_bytes()[0]);
}
#[inline]
pub fn encode_i16(data: &mut Vec<u8>, value: i16) {
	data.extend_from_slice(&value.to_le_bytes());
}
#[inline]
pub fn encode_i32(data: &mut Vec<u8>, value: i32) {
	data.extend_from_slice(&value.to_le_bytes());
}
#[inline]
pub fn encode_i64(data: &mut Vec<u8>, value: i64) {
	data.extend_from_slice(&value.to_le_bytes());
}

#[inline]
pub fn decode_u8(data: &[u8], ind: &mut usize) -> Option<u8> {
	let value = data.get(*ind)?;
	*ind += 1;
	Some(*value)
}
#[inline]
pub fn decode_u16(data: &[u8], ind: &mut usize) -> Option<u16> {
	let value = u16::from_le_bytes(data.get(*ind..*ind + 2)?.try_into().ok()?);
	*ind += 2;
	Some(value)
}
#[inline]
pub fn decode_u32(data: &[u8], ind: &mut usize) -> Option<u32> {
	let value = u32::from_le_bytes(data.get(*ind..*ind + 4)?.try_into().ok()?);
	*ind += 4;
	Some(value)
}
#[inline]
pub fn decode_u64(data: &[u8], ind: &mut usize) -> Option<u64> {
	let value = u64::from_le_bytes(data.get(*ind..*ind + 8)?.try_into().ok()?);
	*ind += 8;
	Some(value)
}

#[inline]
pub fn decode_i8(data: &[u8], ind: &mut usize) -> Option<i8> {
	let value = i8::from_le_bytes([*data.get(*ind)?]);
	*ind += 1;
	Some(value)
}
#[inline]
pub fn decode_i16(data: &[u8], ind: &mut usize) -> Option<i16> {
	let value = i16::from_le_bytes(data.get(*ind..*ind + 2)?.try_into().ok()?);
	*ind += 2;
	Some(value)
}
#[inline]
pub fn decode_i32(data: &[u8], ind: &mut usize) -> Option<i32> {
	let value = i32::from_le_bytes(data.get(*ind..*ind + 4)?.try_into().ok()?);
	*ind += 4;
	Some(value)
}
#[inline]
pub fn decode_i64(data: &[u8], ind: &mut usize) -> Option<i64> {
	let value = i64::from_le_bytes(data.get(*ind..*ind + 8)?.try_into().ok()?);
	*ind += 8;
	Some(value)
}

#[inline]
pub fn encode_f32(data: &mut Vec<u8>, value: f32) {
	data.extend_from_slice(&value.to_le_bytes());
}
#[inline]
pub fn encode_f64(data: &mut Vec<u8>, value: f64) {
	data.extend_from_slice(&value.to_le_bytes());
}

#[inline]
pub fn decode_f32(data: &[u8], ind: &mut usize) -> Option<f32> {
	let value = f32::from_le_bytes(data.get(*ind..*ind + 4)?.try_into().ok()?);
	*ind += 4;
	Some(value)
}
#[inline]
pub fn decode_f64(data: &[u8], ind: &mut usize) -> Option<f64> {
	let value = f64::from_le_bytes(data.get(*ind..*ind + 8)?.try_into().ok()?);
	*ind += 8;
	Some(value)
}

pub fn encode_vuint(data: &mut Vec<u8>, mut value: u64) {
	let mut cond = true;
	// while there is input
	while cond {
		// extract least significant 7 bits
		let byte = value as u8 & 0b0111_1111;
		// shift to next section
		value >>= 7;
		// add continuation bit (0 = end byte)
		data.push(if value == 0 { byte } else { byte | 0b1000_0000 });

		cond = value != 0;
	}
}
pub fn encode_vint(data: &mut Vec<u8>, mut value: i64) {
	let mut cond = true;
	// while there is input
	while cond {
		// extract least significant 7 bits
		let mut byte = (value & 0b0111_1111) as u8;
		// shift to next section
		value >>= 7;
		// ensure at least 1 sign bit is encoded (0 for positive and 1 for negative)
		let sign_bit = byte & 0b0100_0000;
		if (value == 0 && sign_bit == 0) || (value == -1 && sign_bit != 0) {
			cond = false;
		} else {
			// add continuation bit (0 = end byte)
			byte |= 0b1000_0000;
		}
		data.push(byte);
	}
}

pub fn decode_vuint(data: &[u8], ind: &mut usize) -> Option<u64> {
	let mut cond = true;
	let mut res = 0u64;
	let mut shift = 0;

	// while there is input
	while cond {
		let byte = *data.get(*ind)? as u64;
		// add the least significant 7 bits to the next section of the result
		res |= (byte & 0b0111_1111) << shift;
		// next section
		shift += 7;
		*ind += 1;
		// if the continuation bit is set, continue
		cond = byte & 0b1000_0000 != 0;
	}

	Some(res)
}
pub fn decode_vint(data: &[u8], ind: &mut usize) -> Option<i64> {
	let mut cond = true;
	let mut res = 0i64;
	let mut shift = 0u64;
	let mut byte = 0i64;

	// while there is input
	while cond {
		// add the least significant 7 bits to the next section of the result
		byte = *data.get(*ind)? as i64;
		res |= (byte & 0b0111_1111) << shift;
		// next section
		shift += 7;
		*ind += 1;
		// if the continuation bit is set, continue
		cond = byte & 0b1000_0000 != 0;
	}

	// if the value is neg, sign extend it
	if (shift < 64) && (byte & 0b0100_0000 != 0) {
		res |= !0 << shift;
	}

	Some(res)
}
