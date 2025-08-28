use chrono::{DateTime, TimeDelta, Timelike};

use crate::{
	Error, Value,
	errors::unexpected_token,
	parser::utils::{all_matching, is_hex, while_matching},
};

fn parse_uuid_part(source: &str, uuid: &mut [u8; 16], ind: usize) {
	// loop 2 chars at time
	for i in 0..source.len() >> 1 {
		// parse 2 chars into 1 byte of uuid
		uuid[ind + i] = u8::from_str_radix(&source[i * 2..i * 2 + 2], 16).unwrap()
	}
}
pub fn parse_uuid(source: &str, ind: usize) -> Result<Value, Error> {
	// invalid length
	if source.len() != 36 {
		return Err(Error::SyntaxError(format!("invalid uuid ({source}) at {ind}")));
	}

	// extract parts
	let first_part = &source[0..8];
	let second_part = &source[9..13];
	let third_part = &source[14..18];
	let fourth_part = &source[19..23];
	let fifth_part = &source[24..36];

	// check dashes
	if source.as_bytes()[8] != b'-'
		|| source.as_bytes()[13] != b'-'
		|| source.as_bytes()[18] != b'-'
		|| source.as_bytes()[23] != b'-'
	{
		return Err(Error::SyntaxError(format!("invalid uuid ({source}) at {ind}")));
	}

	// all parts are hex
	if !all_matching(first_part, is_hex)
		|| !all_matching(second_part, is_hex)
		|| !all_matching(third_part, is_hex)
		|| !all_matching(fourth_part, is_hex)
		|| !all_matching(fifth_part, is_hex)
	{
		return Err(Error::SyntaxError(format!("invalid uuid ({source}) at {ind}")));
	}

	// parse
	let mut uuid: [u8; 16] = [0; 16];

	parse_uuid_part(first_part, &mut uuid, 0);
	parse_uuid_part(second_part, &mut uuid, 4);
	parse_uuid_part(third_part, &mut uuid, 6);
	parse_uuid_part(fourth_part, &mut uuid, 8);
	parse_uuid_part(fifth_part, &mut uuid, 10);

	Ok(Value::UUID(uuid))
}

pub fn parse_inst(source: &str, nanoseconds: bool, ind: usize) -> Result<Value, Error> {
	let inst = DateTime::parse_from_rfc3339(source).map_err(|_| {
		Error::SyntaxError(format!(
			"invalid {} ({source}) at {ind}",
			if nanoseconds { "instN" } else { "inst" }
		))
	})?;

	// specifing nanoseconds in inst
	if inst.nanosecond() % 1000000 != 0 && !nanoseconds {
		return Err(Error::SyntaxError(format!("invalid inst ({source}) at {ind}")));
	}

	Ok(Value::Inst(inst.with_timezone(&chrono::Utc)))
}

struct DurParseCTX<'a> {
	val: i64,
	parts: Vec<(usize, &'a str)>,
	ind: usize,
	is_first: bool,
	start_ind: usize,
	source: &'a str,
}
fn parse_dur_part(
	ctx: &mut DurParseCTX, unit: &str, multiplier: u64, max: u64,
) -> Result<(), Error> {
	let DurParseCTX { val, parts, ind, is_first, .. } = ctx;

	// skip if not input
	if *ind == parts.len() {
		return Ok(());
	}

	// get number
	let (part_ind, part) = parts[*ind];
	let nb_end = while_matching(part, 0, |c| matches!(c, '0'..='9'));
	if nb_end == 0 {
		return Err(unexpected_token(part.as_bytes()[0] as char, part_ind));
	}

	// skip if not the same unit
	let suffix = &part[nb_end..];
	if suffix != unit {
		if unit == "ns" {
			return Err(unexpected_token(suffix, part_ind + nb_end));
		}
		return Ok(());
	}

	// if number is so large
	let nb = u64::from_str_radix(&part[0..nb_end], 10);
	if nb.is_err() {
		return Err(Error::SyntaxError(format!(
			"duration part ({part}) is so large at {part_ind}",
		)));
	}
	let nb = nb.unwrap();

	// check range if not the first part
	if !*is_first && nb >= max {
		return Err(Error::SyntaxError(format!(
			"duration part ({part}) is out of range 0..{} at {part_ind}",
			max - 1
		)));
	}

	// add while remaining in range
	let added = nb.checked_mul(multiplier);
	if added.is_none() {
		return Err(Error::SyntaxError(format!(
			"duration part ({part}) is so large at {part_ind}",
		)));
	}
	let new_val = val.checked_add_unsigned(added.unwrap());
	if new_val.is_none() {
		return Err(Error::SyntaxError(format!(
			"duration ({}) is so large at {}",
			ctx.source, ctx.start_ind
		)));
	}
	*val = new_val.unwrap();

	*ind += 1;
	*is_first = false;

	Ok(())
}
pub fn parse_dur(mut source: &str, start_ind: usize, src_ind: usize) -> Result<Value, Error> {
	// case negative
	let neg = source.starts_with("-");
	if neg {
		source = &source[1..];
	}

	// split parts by whitespace
	let parts: Vec<(usize, &str)> = source
		.split(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
		.scan(src_ind + 1, |ind, str| {
			let cur = *ind;
			*ind += str.len() + 1;
			Some((cur, str))
		})
		.filter(|(_, str)| !str.is_empty())
		.collect();

	if parts.is_empty() {
		return Err(Error::SyntaxError(format!("invalid duration ({source}) at {start_ind}")));
	}

	let mut ctx = DurParseCTX { val: 0, parts, ind: 0, is_first: true, start_ind, source };

	// parse by units for largest to smallest
	parse_dur_part(&mut ctx, "y", 31536000000000000, 300)?;
	parse_dur_part(&mut ctx, "mn", 2592000000000000, 12)?;
	parse_dur_part(&mut ctx, "d", 86400000000000, 30)?;
	parse_dur_part(&mut ctx, "h", 3600000000000, 24)?;
	parse_dur_part(&mut ctx, "m", 60000000000, 60)?;
	parse_dur_part(&mut ctx, "s", 1000000000, 60)?;
	parse_dur_part(&mut ctx, "ms", 1000000, 1000)?;
	parse_dur_part(&mut ctx, "us", 1000, 1000)?;
	parse_dur_part(&mut ctx, "ns", 1, 1000)?;

	// create duration
	let dur = TimeDelta::new(
		ctx.val / 1_000_000_000 * if neg { -1 } else { 1 },
		(ctx.val % 1_000_000_000) as u32,
	);
	if dur.is_none() {
		return Err(Error::SyntaxError(format!("invalid dur at {start_ind}")));
	}
	return Ok(Value::Dur(dur.unwrap()));
}
