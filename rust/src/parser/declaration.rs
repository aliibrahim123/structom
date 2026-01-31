use std::collections::HashMap;

use crate::{
	DeclFile, DeclProvider, ParseError,
	builtins::{ARR_TYPEID, BUILT_INS_IDS, MAP_TYPEID},
	declaration::{DeclItem, EnumVariant, Field, StructDef, TypeId},
	errors::{ImportError, err},
	parser::{
		ParseOptions,
		tokenizer::Token,
		utils::{
			consume_ident, consume_str, consume_symbol, consume_uint, count_prefix, end_of_input,
			parse_struct_like, remove_n_suffix, try_consume_ident, try_consume_symbol,
			unexpected_token,
		},
	},
};

/// variables used during declaration parsing
pub struct DeclContext<'a> {
	pub provider: &'a dyn DeclProvider,
	pub file: &'a mut DeclFile,
	pub no_ns_imports: Vec<&'a DeclFile>,
	pub ns_imports: HashMap<&'a str, &'a DeclFile>,
	/// current named item id
	pub cur_id: u16,
}

impl<'a> DeclContext<'a> {
	pub fn new(file: &'a mut DeclFile, provider: &'a dyn DeclProvider) -> Self {
		Self { file, no_ns_imports: Vec::new(), ns_imports: HashMap::new(), cur_id: 0, provider }
	}
}

// resolve tag, also parse [tag] specifier
fn resolve_tag(
	tokens: &[Token], ind: &mut usize, cur_tag: &mut u64, tag_type: &str, max_tag: u64,
	item_type: &str, item_name: &str, ctx: &mut DeclContext<'_>,
) -> Result<u32, ParseError> {
	let pos = tokens[*ind].pos();
	let file = &ctx.file.name;
	if *cur_tag > max_tag {
		let msg = format!(
			"maximum number of {tag_type}s ({max_tag}) reached at {item_type} \"{item_name}\"",
		);
		return err!(msg, pos, file);
	}
	let mut tag = *cur_tag;

	if try_consume_symbol('[', tokens, ind, file)? {
		let spec_tag = consume_uint(tokens, ind, &ctx.file.name)?;

		if spec_tag < *cur_tag as u64 {
			let msg = format!(
				"{tag_type} ({spec_tag}) must be at least ({cur_tag}) at {item_type} \"{item_name}\"",
			);
			return err!(msg, pos, file);
		}
		if spec_tag > max_tag {
			let msg = format!(
				"{tag_type} ({spec_tag}) is greater than maximum allowed value ({max_tag}) at {item_type} \"{item_name}\""
			);
			return err!(msg, pos, file);
		}
		tag = spec_tag;

		consume_symbol(']', tokens, ind, file)?;
	}

	*cur_tag = tag + 1;
	Ok(tag as u32)
}

fn parse_import<'a>(
	tokens: &'a [Token], ind: &mut usize, imports: &mut Vec<u64>, ctx: &mut DeclContext<'a>,
	options: &ParseOptions,
) -> Result<(), ParseError> {
	let pos = tokens[*ind].pos();
	*ind += 1; // skip "import"

	let cur_file = &ctx.file.name;

	let mut path = consume_str(tokens, ind, cur_file)?;
	// resolve path
	let path_owner;
	let is_cur_dir = path.starts_with("./");
	let is_parent_dir = path.starts_with("../");
	if options.relative_paths && (is_cur_dir || is_parent_dir) {
		path_owner = Some(if is_cur_dir {
			remove_n_suffix(&cur_file, "/", 1).to_string() + "/" + path.strip_prefix("./").unwrap()
		} else {
			let up_dirs = count_prefix(path, "../") + 1;
			remove_n_suffix(&cur_file, "/", up_dirs).to_string()
				+ "/" + path.trim_start_matches("../")
		});
		path = &path_owner.as_ref().unwrap();
	}

	let imported = match ctx.provider.load(path) {
		Ok(file) => file,
		Err(ImportError::NotFound) => {
			return err!(format!("declaration file \"{path}\" not found"), pos, cur_file);
		}
		Err(ImportError::Parse(error)) => return Err(error),
		Err(ImportError::Other(error)) => {
			return err!(format!("while importing \"{path}\" encountered: {error}"), pos, cur_file);
		}
	};

	if imports.contains(&imported.id) {
		return err!(format!("importing declaration file \"{path}\" twice"), pos, cur_file);
	}
	imports.push(imported.id);

	if matches!(try_consume_ident(tokens, ind, cur_file)?, Some("as")) {
		let ns = consume_ident(tokens, ind, cur_file)?;
		if ctx.ns_imports.contains_key(ns) {
			let msg = format!("importing \"{path}\" into used namespace \"{ns}\"");
			return err!(msg, pos, cur_file);
		}
		if ctx.file.get_by_name(ns).is_some() {
			let msg =
				format!("importing \"{path}\" into namespace named like existing item \"{ns}\"");
			return err!(msg, pos, cur_file);
		}
		ctx.ns_imports.insert(ns, imported);
	} else {
		ctx.no_ns_imports.push(imported)
	}

	Ok(())
}

fn parse_anonymous_item(
	tokens: &[Token], ind: &mut usize, metadata: Option<Vec<(String, String)>>,
	ctx: &mut DeclContext, options: &ParseOptions,
) -> Result<TypeId, ParseError> {
	let typeid = ctx.cur_id;
	ctx.cur_id += 1;
	match consume_ident(tokens, ind, &ctx.file.name)? {
		"struct" => {
			let name = format!("anonymous_struct_{typeid:x}");
			let def = parse_fields(tokens, ind, &name, ctx, options)?;
			ctx.file.add_item(DeclItem::Struct { name, typeid, def });
		}
		"enum" => {
			let name = format!("anonymous_enum_{typeid:x}");
			let mut decl = DeclItem::new_enum(name, typeid);
			parse_enum_body(tokens, ind, &mut decl, ctx, options)?;
			_ = ctx.file.add_item(decl);
		}
		_ => unreachable!(),
	}
	return Ok(TypeId::new(ctx.file.id, typeid, metadata));
}

pub fn parse_metadata(
	tokens: &[Token], ind: &mut usize, options: &ParseOptions, file: &str,
) -> Result<Option<Vec<(String, String)>>, ParseError> {
	let mut metadata = None;
	while try_consume_symbol('@', tokens, ind, file)? {
		let pos = tokens[*ind - 1].pos();

		let name = consume_ident(tokens, ind, file)?;
		consume_symbol('(', tokens, ind, file)?;
		let value = consume_str(tokens, ind, file)?;
		consume_symbol(')', tokens, ind, file)?;

		let metadata = match &mut metadata {
			None if options.metadata == false => continue,
			Some(metadata) => metadata,
			None => {
				metadata = Some(vec![]);
				metadata.as_mut().unwrap()
			}
		};

		if metadata.iter().any(|(n, _)| name == n) {
			return err!(format!("declaring a metadata \"{name}\" multiple times"), pos, file);
		}

		metadata.push((name.to_string(), value.to_string()));
	}
	Ok(metadata)
}

// macro since it depend on a specific form of parse_typeid
// decleration take mut ctx since it add inline structs, while value not
macro_rules! parse_typeid_general {
	($args:expr) => {{
		let (tokens, ind, metadata, ctx, options) = $args;

		let file = &ctx.file.name;
		let provider = ctx.provider;
		let pos = tokens[*ind].pos();
		let type_name = consume_ident(tokens, ind, file)?;

		if type_name == "arr" {
			consume_symbol('<', tokens, ind, file)?;
			let itemid = parse_typeid(tokens, ind, ctx, options)?;
			consume_symbol('>', tokens, ind, &ctx.file.name)?;
			return Ok(TypeId::with_variant(0, ARR_TYPEID, 0, Some(itemid), metadata));
		}

		if type_name == "map" {
			consume_symbol('<', tokens, ind, file)?;
			let keyid = parse_typeid(tokens, ind, ctx, options)?;
			if (!keyid.is_builtin() || keyid.id == ARR_TYPEID || keyid.id == MAP_TYPEID) {
				let msg = format!("map key must be primitive, got: {}", keyid.name(provider));
				return err!(msg, pos, &ctx.file.name);
			}

			consume_symbol(',', tokens, ind, &ctx.file.name)?;
			let valueid = parse_typeid(tokens, ind, ctx, options)?;
			consume_symbol('>', tokens, ind, &ctx.file.name)?;

			return Ok(TypeId::with_variant(0, MAP_TYPEID, keyid.id, Some(valueid), metadata));
		}

		if let Some(id) = BUILT_INS_IDS.get(type_name) {
			return Ok(TypeId::new(0, *id, metadata));
		}

		if let Some(item) = ctx.file.get_by_name(type_name) {
			return Ok(TypeId::new(ctx.file.id, item.typeid(), metadata));
		}

		for file in ctx.no_ns_imports.iter() {
			if let Some(item) = file.get_by_name(type_name) {
				return Ok(TypeId::new(file.id, item.typeid(), metadata));
			}
		}

		if try_consume_symbol('.', tokens, ind, file)? {
			let ns = type_name;
			let type_name = consume_ident(tokens, ind, file)?;

			let Some(ns_file) = ctx.ns_imports.get(ns) else {
				return err!(format!("undefined namespace \"{ns}\""), pos, file);
			};
			let Some(item) = ns_file.get_by_name(type_name) else {
				let msg = format!("undefined type \"{type_name}\" in namespace \"{ns}\"");
				return err!(msg, pos, file);
			};
			return Ok(TypeId::new(ns_file.id, item.typeid(), metadata));
		}

		return err!(format!("undefined type \"{type_name}\""), pos, file);
	}};
}
pub(crate) use parse_typeid_general;

fn parse_typeid(
	tokens: &[Token], ind: &mut usize, ctx: &mut DeclContext<'_>, options: &ParseOptions,
) -> Result<TypeId, ParseError> {
	let file = &ctx.file.name;
	let metadata = parse_metadata(tokens, ind, options, file)?;

	if matches!(tokens[*ind], Token::Ident("struct" | "enum", _)) {
		return parse_anonymous_item(tokens, ind, metadata, ctx, options);
	}

	parse_typeid_general!((tokens, ind, metadata, ctx, options))
}

fn parse_fields(
	tokens: &[Token], ind: &mut usize, item: &str, ctx: &mut DeclContext<'_>,
	options: &ParseOptions,
) -> Result<StructDef, ParseError> {
	let start_pos = tokens[*ind].pos();
	let mut def = StructDef::default();
	let mut cur_tag = 0;

	parse_struct_like!((tokens, '{', '}'), &ctx.file.name, ind => {
		let pos = tokens[*ind].pos();

		// named like that to prevent an arguments tower
		const MTAG: u64 = u32::MAX as u64;
		let tag = resolve_tag(tokens, ind, &mut cur_tag, "field tag", MTAG, "struct", item, ctx)?;

		let file = &ctx.file.name;

		let name = match tokens.get(*ind) {
			Some(Token::Ident(ident, _)) => ident.to_string(),
			Some(Token::Str(str, _)) => str.to_string(),
			Some(Token::EOF(_)) | None => return end_of_input(file),
			Some(token) => return unexpected_token(token, token.pos(), file),
		};
		*ind += 1;

		if def.get_field_by_name(&name).is_some() {
			let msg = format!("declaring a field \"{name}\" mutliple times at struct \"{item}\"");
			return err!(msg, pos, file);
		}

		let is_optional = try_consume_symbol('?', tokens, ind, file)?;
		consume_symbol(':', tokens, ind, file)?;

		let typeid = parse_typeid(tokens, ind, ctx, options)?;

		def.add_field(Field { name, tag, typeid, is_optional });
	});

	if def.fields.is_empty() {
		let msg = format!("struct \"{item}\" must have at least one field");
		return err!(msg, start_pos, &ctx.file.name);
	}

	return Ok(def);
}

fn parse_enum_body(
	tokens: &[Token], ind: &mut usize, decl: &mut DeclItem, ctx: &mut DeclContext<'_>,
	options: &ParseOptions,
) -> Result<(), ParseError> {
	let start_pos = tokens[*ind].pos();
	let mut cur_tag = 0;

	parse_struct_like!((tokens, '{', '}'), &ctx.file.name, ind => {
		let name = decl.name();
		let pos = tokens[*ind].pos();

		// named like that to prevent an arguments tower
		const MTAG: u64 = u32::MAX as u64;
		let tag = resolve_tag(tokens, ind, &mut cur_tag, "variant tag", MTAG, "enum", name, ctx)?;

		let file = &ctx.file.name;

		let variant = consume_ident(tokens, ind, file)?;
		if decl.get_variant_by_name(variant).is_some() {
			let msg = format!("declaring variant \"{variant}\" mutliple times at enum \"{name}\"");
			return err!(msg, pos, file);
		}

		let field_def = match tokens.get(*ind) {
			Some(Token::Symbol('{', _)) => {
				Some(parse_fields(tokens, ind, &format!("{name}.{variant}"), ctx, options)?)
			}
			_ => None,
		};

		decl.add_variant(EnumVariant { name: variant.to_string(), tag, def: field_def });
	});

	// ensure at least one variant
	if let DeclItem::Enum { variants, .. } = &decl
		&& variants.is_empty()
	{
		let msg = format!("enum \"{}\" must have at least one variant", decl.name());
		return err!(msg, start_pos, &ctx.file.name);
	}

	Ok(())
}

fn parse_item_common<'a>(
	tokens: &'a [Token], ind: &mut usize, ctx: &mut DeclContext<'a>,
) -> Result<(&'a str, u16), ParseError> {
	*ind += 1; // skip struct / enum

	let pos = tokens[*ind].pos();
	let file_name = &ctx.file.name;

	let name = consume_ident(tokens, ind, file_name)?;
	if ctx.file.get_by_name(name).is_some() {
		return err!(format!("declaring item \"{name}\" mutliple times"), pos, file_name);
	}
	if ctx.ns_imports.contains_key(name) {
		let msg = format!("declaring item \"{name}\" with name similar to existing namespace");
		return err!(msg, pos, file_name);
	}

	let mut cur_id = ctx.cur_id as u64;
	let id = resolve_tag(tokens, ind, &mut cur_id, "item id", u16::MAX as u64, "item", name, ctx)?;
	ctx.cur_id = cur_id as u16;

	Ok((name, id as u16))
}

pub fn parse_declaration<'a>(
	file: &'a mut DeclFile, tokens: &'a [Token], ind: &mut usize, provider: &'a dyn DeclProvider,
	options: &ParseOptions,
) -> Result<DeclContext<'a>, ParseError> {
	let mut ctx = DeclContext::new(file, provider);
	let mut imports = Vec::<u64>::new();

	while let Some(Token::Ident(ident, _)) = tokens.get(*ind) {
		match *ident {
			"import" => parse_import(tokens, ind, &mut imports, &mut ctx, options)?,
			"struct" => {
				let (name, id) = parse_item_common(tokens, ind, &mut ctx)?;
				let def = parse_fields(tokens, ind, name, &mut ctx, options)?;
				_ = ctx.file.add_item(DeclItem::Struct { name: name.to_string(), typeid: id, def });
			}
			"enum" => {
				let (name, id) = parse_item_common(tokens, ind, &mut ctx)?;
				let mut decl = DeclItem::new_enum(name.to_string(), id);
				parse_enum_body(tokens, ind, &mut decl, &mut ctx, options)?;
				_ = ctx.file.add_item(decl);
			}
			_ => break,
		}
	}

	Ok(ctx)
}
