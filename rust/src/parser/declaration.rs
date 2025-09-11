use std::collections::HashMap;

use crate::{
	DeclFile, DeclProvider, ParserError,
	builtins::BUILT_INS_IDS,
	declaration::{DeclItem, EnumVariant, Field, StructDef, TypeId},
	errors::{end_of_input, unexpected_token},
	parser::{
		ParseOptions,
		tokenizer::Token,
		utils::{
			consume_ident, consume_str, consume_symbol, consume_uint, struct_like_end,
			struct_like_start,
		},
	},
};

pub struct DeclContext<'a> {
	pub file: &'a mut DeclFile,
	pub no_ns_imports: Vec<&'a DeclFile>,
	pub ns_imports: HashMap<&'a str, &'a DeclFile>,
	pub cur_low_id: u16,
	pub cur_high_id: u16,
}

impl<'a> DeclContext<'a> {
	pub fn new(file: &'a mut DeclFile) -> Self {
		Self {
			file,
			no_ns_imports: Vec::new(),
			ns_imports: HashMap::new(),
			cur_low_id: 0,
			cur_high_id: u16::MAX,
		}
	}
}

// parse [tag]
fn parse_tag(
	tokens: &[Token], ind: &mut usize, cur_tag: &mut u32, tag_name: &str, item: &str,
	item_name: &str, ctx: &mut DeclContext<'_>,
) -> Result<u32, ParserError> {
	let mut tag = *cur_tag as u64;

	if let Some(Token::Symbol('[', _)) = tokens.get(*ind) {
		*ind += 1;

		tag = consume_uint(tokens, ind)?;
		// specified tag only allowed to be greater than previous
		if tag < *cur_tag as u64 {
			return Err(ParserError::TypeError(format!(
				"{tag_name} ({tag}) must be greater than previous {tag_name} ({}) at {item} \"{item_name}\" in declaration file \"{}\"",
				*cur_tag - 1,
				ctx.file.name
			)));
		}

		consume_symbol(']', tokens, ind)?;
	}
	if tag > 0xffffffff {
		return Err(ParserError::TypeError(format!(
			"{tag_name} ({tag}) is greater than maximum allowed value (0xffffff) at {item} \"{item_name}\" in declaration file \"{}\"",
			ctx.file.name
		)));
	}
	*cur_tag = tag as u32 + 1;
	Ok(tag as u32)
}

fn parse_import<'a>(
	tokens: &'a [Token], ind: &mut usize, imports: &mut Vec<u64>, ctx: &mut DeclContext<'a>,
	provider: &'a dyn DeclProvider,
) -> Result<(), ParserError> {
	// skip "import"
	*ind += 1;

	let cur_file_name = &ctx.file.name;

	let path = consume_str(tokens, ind)?;
	let file = provider.get_by_name(path);
	if file.is_none() {
		return Err(ParserError::TypeError(format!(
			"unable to import declaration file \"{path}\" at declaration file \"{cur_file_name}\"",
		)));
	}
	let file = file.unwrap();

	if imports.contains(&file.id) {
		return Err(ParserError::SyntaxError(format!(
			"importing declaration file \"{path}\" twice at declaration file \"{cur_file_name}\""
		)));
	}
	imports.push(file.id);

	// check for specified namespace
	match tokens.get(*ind) {
		Some(Token::Identifier("as", _)) => {
			// skip "as"
			*ind += 1;

			let ns = consume_ident(tokens, ind)?;

			// check for namespace collision
			if ctx.ns_imports.contains_key(ns) {
				return Err(ParserError::TypeError(format!(
					"importing a declaration file \"{path}\" into used namespace \"{ns}\" at declaration file \"{cur_file_name}\""
				)));
			}

			ctx.ns_imports.insert(ns, file);
		}
		_ => ctx.no_ns_imports.push(file),
	};

	Ok(())
}

fn parse_anonymous_item(
	type_name: &str, tokens: &[Token], ind: &mut usize, ctx: &mut DeclContext,
	options: &ParseOptions, metadata: Option<Vec<(String, String)>>,
) -> Result<TypeId, ParserError> {
	match type_name {
		"struct" => {
			let typeid = ctx.cur_high_id;
			ctx.cur_high_id -= 1;
			let name = format!("anonymous_struct_{typeid:x}");

			let def = parse_fields(tokens, ind, &name, ctx, options)?;

			_ = ctx.file.add_item(DeclItem::Struct { name, typeid, def });

			return Ok(TypeId::new(ctx.file.id, typeid, metadata));
		}
		"enum" => {
			let typeid = ctx.cur_high_id;
			ctx.cur_high_id -= 1;
			let name = format!("anonymous_enum_{typeid:x}");

			let mut decl = DeclItem::new_enum(name.to_string(), typeid);

			parse_enum_body(tokens, ind, &mut decl, &name, ctx, options)?;

			_ = ctx.file.add_item(decl);

			return Ok(TypeId::new(ctx.file.id, typeid, metadata));
		}
		_ => unreachable!(),
	}
}

pub fn parse_metadata(
	tokens: &[Token], ind: &mut usize, loc: &impl Fn() -> String, options: &ParseOptions,
) -> Result<Option<Vec<(String, String)>>, ParserError> {
	let mut metadata = None;

	// while there metadata
	while let Some(Token::Symbol('@', _)) = tokens.get(*ind) {
		*ind += 1;

		// consume name(value)
		let name = consume_ident(tokens, ind)?;
		consume_symbol('(', tokens, ind)?;
		let value = consume_str(tokens, ind)?;
		consume_symbol(')', tokens, ind)?;

		if options.metadata && metadata.is_none() {
			metadata = Some(vec![]);
		}

		if let Some(metadata) = metadata.as_mut() {
			// check for metadata collision
			if metadata.iter().any(|(n, _)| name == n) {
				return Err(ParserError::TypeError(format!(
					"declaring multiple metadata with the same name \"{name}\" {}",
					loc()
				)));
			}
			metadata.push((name.to_string(), value.to_string()));
		}
	}

	Ok(metadata)
}

// macro since it depend on a specific form of parse_typeid, one for
// declarations and one for values
macro_rules! parse_typeid_general {
	($args:expr, $loc:expr) => {{
		let (tokens, ind, type_name, loc, metadata, ctx, options) = $args;
		// array
		if type_name == "arr" {
			consume_symbol('<', tokens, ind)?;
			let itemid = parse_typeid(tokens, ind, loc, ctx, options)?;
			consume_symbol('>', tokens, ind)?;

			return Ok(TypeId::with_variant(0, 0x22, 0, Some(itemid), metadata));
		}

		// map
		if type_name == "map" {
			consume_symbol('<', tokens, ind)?;

			let keyid = parse_typeid(tokens, ind, loc, ctx, options)?;
			if (keyid.ns != 0 || keyid.id == 0x23 || keyid.id == 0x22) {
				return Err(ParserError::TypeError(format!(
					"map key can only be a primitive {}",
					$loc(ctx)
				)));
			}

			consume_symbol(',', tokens, ind)?;
			let valueid = parse_typeid(tokens, ind, loc, ctx, options)?;
			consume_symbol('>', tokens, ind)?;

			return Ok(TypeId::with_variant(0, 0x23, keyid.id, Some(valueid), metadata));
		}

		// check for built-in
		if let Some(id) = BUILT_INS_IDS.get(type_name).cloned() {
			return Ok(TypeId::new(0, id, metadata));
		}

		// check for current file
		if let Some(item) = ctx.file.get_by_name(type_name) {
			return Ok(TypeId::new(ctx.file.id, item.typeid(), metadata));
		}

		// check for no-namespace imports
		let mut files = ctx.no_ns_imports.iter();
		if let Some((file, item)) = files.find_map(|f| f.get_by_name(type_name).map(|i| (f, i))) {
			return Ok(TypeId::new(file.id, item.typeid(), metadata));
		}

		// namespaced items
		if let Some(Token::Symbol('.', _)) = tokens.get(*ind) {
			let ns = type_name;
			*ind += 1; // skip '.'

			let type_name = consume_ident(tokens, ind)?;

			// get the file of the namespace
			let file = ctx.ns_imports.get(ns);
			if file.is_none() {
				return Err(ParserError::TypeError(format!(
					"undefined namespace \"{ns}\" {}",
					$loc(ctx)
				)));
			};
			let file = file.unwrap();

			// get the item
			let item = file.get_by_name(type_name);
			if item.is_none() {
				return Err(ParserError::TypeError(format!(
					"undefined type \"{type_name}\" in namespace \"{ns}\" {}",
					$loc(ctx)
				)));
			}

			return Ok(TypeId::new(file.id, item.unwrap().typeid(), metadata));
		}

		Err(ParserError::TypeError(format!("undefined type \"{type_name}\" {}", $loc(ctx))))
	}};
}
pub(crate) use parse_typeid_general;

fn parse_typeid(
	tokens: &[Token], ind: &mut usize, loc: &impl Fn(&DeclContext<'_>) -> String,
	ctx: &mut DeclContext<'_>, options: &ParseOptions,
) -> Result<TypeId, ParserError> {
	let metadata = parse_metadata(tokens, ind, &|| loc(ctx), options)?;

	let first_part = consume_ident(tokens, ind)?;

	// case anonymous items
	match first_part {
		"struct" | "enum" => {
			return parse_anonymous_item(first_part, tokens, ind, ctx, options, metadata);
		}
		_ => (),
	}

	parse_typeid_general!((tokens, ind, first_part, loc, metadata, ctx, options), |ctx| loc(ctx))
}

fn parse_fields(
	tokens: &[Token], ind: &mut usize, item: &str, ctx: &mut DeclContext<'_>,
	options: &ParseOptions,
) -> Result<StructDef, ParserError> {
	let mut def = StructDef::default();
	let mut cur_tag = 0u32;
	let mut watched_comma = true;

	consume_symbol('{', tokens, ind)?;

	// loop through fields
	loop {
		if struct_like_start(tokens, ind, &mut watched_comma, '}')? {
			break;
		}

		let tag = parse_tag(tokens, ind, &mut cur_tag, "field tag", "struct", item, ctx)?;

		let name = match tokens.get(*ind) {
			Some(Token::Identifier(ident, _)) => ident.to_string(),
			Some(Token::Str(str, _)) => str.to_string(),
			Some(Token::EOF(_)) | None => return Err(end_of_input(tokens[*ind].ind())),
			Some(token) => return Err(unexpected_token(token, token.ind())),
		};
		*ind += 1;

		// check for collision
		if def.get_field_by_name(&name).is_some() {
			return Err(ParserError::TypeError(format!(
				"declaring multiple fields with the same name \"{name}\" at struct \"{item}\" in declaration file \"{}\"",
				ctx.file.name
			)));
		}

		let is_optional = matches!(tokens.get(*ind), Some(Token::Symbol('?', _)));
		is_optional.then(|| *ind += 1);

		consume_symbol(':', tokens, ind)?;

		let loc = |ctx: &DeclContext| {
			format!(
				"at field \"{name}\" in struct \"{item}\" in declaration file \"{}\"",
				ctx.file.name
			)
		};
		let typeid = parse_typeid(tokens, ind, &loc, ctx, options)?;

		_ = def.add_field(Field::new(name, tag, typeid, is_optional));

		struct_like_end(tokens, ind, &mut watched_comma);
	}

	// ensure at least one field
	if def.fields.is_empty() {
		return Err(ParserError::TypeError(format!(
			"struct \"{item}\" must have at least one field at declaration file \"{}\"",
			ctx.file.name
		)));
	}

	return Ok(def);
}

fn parse_enum_body(
	tokens: &[Token], ind: &mut usize, decl: &mut DeclItem, name: &str, ctx: &mut DeclContext<'_>,
	options: &ParseOptions,
) -> Result<(), ParserError> {
	let mut watched_comma = true;
	let mut cur_tag = 0u32;

	consume_symbol('{', tokens, ind)?;

	// loop through variants
	loop {
		if struct_like_start(tokens, ind, &mut watched_comma, '}')? {
			break;
		}

		let tag = parse_tag(tokens, ind, &mut cur_tag, "variant tag", "enum", name, ctx)?;

		let variant = consume_ident(tokens, ind)?;
		if decl.get_variant_by_name(variant).is_some() {
			return Err(ParserError::TypeError(format!(
				"declaring multiple variants with the same name \"{variant}\" at enum \"{name}\" in declaration file \"{}\"",
				ctx.file.name
			)));
		}

		// case has fields
		let field_def = match tokens.get(*ind) {
			Some(Token::Symbol('{', _)) => Some(parse_fields(tokens, ind, variant, ctx, options)?),
			_ => None,
		};

		_ = decl.add_variant(EnumVariant { name: variant.to_string(), tag, def: field_def });

		struct_like_end(tokens, ind, &mut watched_comma);
	}

	// ensure at least one variant
	if let DeclItem::Enum { variants, .. } = decl
		&& variants.is_empty()
	{
		return Err(ParserError::TypeError(format!(
			"enum \"{name}\" must have at least one variant at declaration file \"{}\"",
			ctx.file.name
		)));
	}

	Ok(())
}

fn parse_item_common<'a>(
	tokens: &'a [Token], ind: &mut usize, ctx: &mut DeclContext<'a>,
) -> Result<(&'a str, u16), ParserError> {
	*ind += 1;

	let file = &ctx.file;
	let file_name = &file.name;
	let mut cur_id = ctx.cur_low_id as u32;

	let name = consume_ident(tokens, ind)?;
	if file.get_by_name(name).is_some() {
		return Err(ParserError::TypeError(format!(
			"declaring multiple item with the same name \"{name}\" at declaration file \"{file_name}\""
		)));
	}

	let id = parse_tag(tokens, ind, &mut cur_id, "item id", "item", name, ctx)?;
	if id > 0xffff {
		return Err(ParserError::TypeError(format!(
			"item id ({id}) is greater than maximum allowed value (0xffff) at item \"{name}\" in declaration file \"{}\"",
			ctx.file.name
		)));
	}
	ctx.cur_low_id = cur_id as u16;

	Ok((name, id as u16))
}

pub fn parse_declaration<'a>(
	file: &'a mut DeclFile, tokens: &'a [Token], ind: &mut usize, provider: &'a dyn DeclProvider,
	options: &ParseOptions,
) -> Result<DeclContext<'a>, ParserError> {
	let mut ctx = DeclContext::new(file);
	let mut imports = Vec::<u64>::new();

	while let Some(Token::Identifier(ident, _)) = tokens.get(*ind) {
		match *ident {
			"import" => parse_import(tokens, ind, &mut imports, &mut ctx, provider)?,
			"struct" => {
				let (name, id) = parse_item_common(tokens, ind, &mut ctx)?;

				let def = parse_fields(tokens, ind, name, &mut ctx, options)?;

				_ = ctx.file.add_item(DeclItem::Struct { name: name.to_string(), typeid: id, def });
			}
			"enum" => {
				let (name, id) = parse_item_common(tokens, ind, &mut ctx)?;

				let mut decl = DeclItem::new_enum(name.to_string(), id);

				parse_enum_body(tokens, ind, &mut decl, name, &mut ctx, options)?;

				_ = ctx.file.add_item(decl);
			}
			_ => break,
		}
	}

	Ok(ctx)
}
