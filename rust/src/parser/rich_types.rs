use chrono::{DateTime, TimeDelta, Timelike};

use crate::{
	ParseError, Value,
	errors::err,
	parser::{
		tokenizer::{Pos, Token},
		utils::{StrExt, all_matching, consume_str, is_hex, while_matching},
	},
};

fn parse_uuid_part(source: &str, uuid: &mut [u8; 16], ind: usize) {
	for i in 0..source.len() >> 1 {
		uuid[ind + i] = u8::from_str_radix(&source[i * 2..i * 2 + 2], 16).unwrap()
	}
}
pub fn parse_uuid(source: &str, pos: Pos, file: &str) -> Result<Value, ParseError> {
	let invalid_uuid = || err!(format!("invalid uuid ({source})"), pos, file);

	if source.len() != 36 {
		return invalid_uuid();
	}

	let first_part = &source[0..8];
	let second_part = &source[9..13];
	let third_part = &source[14..18];
	let fourth_part = &source[19..23];
	let fifth_part = &source[24..36];

	if source.as_bytes()[8] != b'-'
		|| source.as_bytes()[13] != b'-'
		|| source.as_bytes()[18] != b'-'
		|| source.as_bytes()[23] != b'-'
	{
		return invalid_uuid();
	}
	if !all_matching(first_part, is_hex)
		|| !all_matching(second_part, is_hex)
		|| !all_matching(third_part, is_hex)
		|| !all_matching(fourth_part, is_hex)
		|| !all_matching(fifth_part, is_hex)
	{
		return invalid_uuid();
	}

	let mut uuid: [u8; 16] = [0; 16];
	parse_uuid_part(first_part, &mut uuid, 0);
	parse_uuid_part(second_part, &mut uuid, 4);
	parse_uuid_part(third_part, &mut uuid, 6);
	parse_uuid_part(fourth_part, &mut uuid, 8);
	parse_uuid_part(fifth_part, &mut uuid, 10);
	Ok(Value::UUID(uuid))
}

pub fn parse_inst(
	source: &str, nanoseconds: bool, pos: Pos, file: &str,
) -> Result<Value, ParseError> {
	let Ok(inst) = DateTime::parse_from_rfc3339(source) else {
		let msg = format!("invalid {} ({source})", if nanoseconds { "instN" } else { "inst" });
		return err!(msg, pos, file);
	};

	// specifing nanoseconds in inst
	if inst.nanosecond() % 1000000 != 0 && !nanoseconds {
		return err!(format!("invalid inst ({source})"), pos, file);
	}

	Ok(Value::Inst(inst.with_timezone(&chrono::Utc)))
}

struct DurParseCTX<'a> {
	val: i64,
	parts: Vec<(&'a str, usize, Pos)>,
	ind: usize,
	is_first: bool,
	start_pos: Pos,
	file: &'a str,
	source: &'a str,
}
const US_AS_NS: u64 = 1000;
const MS_AS_NS: u64 = 1000 * US_AS_NS;
const S_AS_NS: u64 = 1000 * MS_AS_NS;
const M_AS_NS: u64 = 60 * S_AS_NS;
const H_AS_NS: u64 = 60 * M_AS_NS;
const D_AS_NS: u64 = 24 * H_AS_NS;
const MN_AS_NS: u64 = 30 * D_AS_NS;
const Y_AS_NS: u64 = 365 * D_AS_NS;
fn parse_dur_part(
	ctx: &mut DurParseCTX, unit: &str, multiplier: u64, max: u64,
) -> Result<bool, ParseError> {
	let DurParseCTX { parts, ind, is_first, file, .. } = ctx;

	if *ind == parts.len() {
		return Ok(false);
	}

	let (part, split, pos) = parts[*ind];
	let (amount, suffix) = part.split_at(split);

	if suffix != unit {
		if unit == "ns" {
			return err!(format!("unkown unit ({suffix})"), pos + amount.len(), file);
		}
		return Ok(false);
	}

	let Ok(amount) = u64::from_str_radix(amount, 10) else {
		return err!(format!("duration part ({amount}) is large"), pos, file);
	};

	// first part is not capped
	if !*is_first && amount >= max {
		let msg = format!("duration part ({part}) is out of range 0{unit}..{}{unit}", max - 1);
		return err!(msg, pos, file);
	}

	// can not overflow
	ctx.val += (amount * multiplier) as i64;
	*ind += 1;
	*is_first = false;
	Ok(true)
}
pub fn parse_dur(tokens: &[Token], ind: &mut usize, file: &str) -> Result<Value, ParseError> {
	let start_pos = tokens[*ind - 1].pos();
	let src_pos = tokens[*ind].pos();
	let mut source = consume_str(tokens, ind, file)?;

	let neg = source.starts_with("-");
	if neg {
		source = &source[1..];
	}

	// split by whitespace
	let mut parts = Vec::new();
	let mut last_ind = 0;
	let mut pos = start_pos + 1u32;
	while let Some(ind) = source.find_ws_after(last_ind) {
		let part = &source[last_ind..ind];
		if part != "" {
			let split = while_matching(part, 0, |c| matches!(c, '0'..='9'));
			if split == 0 {
				return err!(format!("invalid duration part ({part})"), pos, file);
			}
			parts.push((part, split, pos));
		}
		if source.char_at(ind) == Some('\n') {
			pos.line += 1;
			pos.col = 1;
		} else {
			pos.col += part.len() as u32 + 1;
		}
		last_ind = ind + 1;
	}

	if parts.is_empty() {
		return err!("empty duration".to_string(), start_pos, file);
	}

	let mut ctx = DurParseCTX { val: 0, parts, ind: 0, is_first: true, start_pos, source, file };
	// 292y 172d overflows i64, and 290 is a good approximation
	parse_dur_part(&mut ctx, "y", Y_AS_NS, 290)?;
	let has_months = parse_dur_part(&mut ctx, "mn", MN_AS_NS, 12)?;
	parse_dur_part(&mut ctx, "d", D_AS_NS, if has_months { 30 } else { 365 })?;
	parse_dur_part(&mut ctx, "h", H_AS_NS, 24)?;
	parse_dur_part(&mut ctx, "m", M_AS_NS, 60)?;
	parse_dur_part(&mut ctx, "s", S_AS_NS, 60)?;
	parse_dur_part(&mut ctx, "ms", MS_AS_NS, 1000)?;
	parse_dur_part(&mut ctx, "us", US_AS_NS, 1000)?;
	parse_dur_part(&mut ctx, "ns", 1, 1000)?;

	let Some(dur) = TimeDelta::new(
		ctx.val / 1_000_000_000 * if neg { -1 } else { 1 },
		(ctx.val % 1_000_000_000) as u32,
	) else {
		return err!(format!("invalid duration ({source})"), start_pos, file);
	};
	return Ok(Value::Dur(dur));
}
