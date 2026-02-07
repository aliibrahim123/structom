mod js;
mod rust;
pub mod utils;

use std::fs::{canonicalize, create_dir, create_dir_all, remove_dir_all};
use std::{fs::read_dir, path::Path};

use clap::{Parser, ValueEnum};
use structom::DeclFile;
use structom::FSProvider;

use crate::js::to_js;
use crate::rust::to_rust;
use crate::utils::errors;

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Lang {
	Rust,
	JS,
}

/// generate serialization code for structom declerations
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
	/// declerations directory path
	#[arg(short, long)]
	input: String,

	/// generated code output path
	#[arg(short, long)]
	output: String,

	/// language of the generated code
	#[arg(short, long)]
	lang: Lang,
}

fn main() {
	if let Err(err) = mainer() {
		eprintln!("{err}");
	}
}
fn mainer() -> Result<(), String> {
	let Args { input, output, lang } = Args::parse();

	create_dir_all(&output).map_err(errors::create_dir(&output))?;
	let out_path = canonicalize(&output).map_err(errors::read_dir(&output))?;
	remove_dir_all(&out_path).map_err(errors::remove_dir(&output))?;
	create_dir(&out_path).map_err(errors::create_dir(&output))?;

	let input = canonicalize(&input).map_err(errors::read_dir(&input))?;
	let provider = FSProvider::new(&input).unwrap();
	let mut inputs = Vec::new();
	collect_decls(&mut inputs, &input, "".to_string(), &provider)?;

	match lang {
		Lang::Rust => to_rust(&inputs, input.to_str().unwrap(), &out_path, &provider)?,
		Lang::JS => to_js(&inputs, input.to_str().unwrap(), &out_path, &provider)?,
	}

	Ok(())
}

#[derive(Debug)]
pub struct Entry<'a> {
	/// relative path in the input directory
	pub rel_path: String,
	/// relative path in the output directory
	pub resolved_path: String,
	pub decl: &'a DeclFile,
}

/// recursively walk the declerations directory and collect declerations
pub fn collect_decls<'a>(
	inputs: &mut Vec<Entry<'a>>, path: &Path, rel_path: String, provider: &'a FSProvider,
) -> Result<(), String> {
	for entry in read_dir(path).map_err(errors::read_dir(&path.display()))? {
		let entry = entry.map_err(errors::read_dir(&path.display()))?.path();

		let file_name = entry.file_name().unwrap().to_str().unwrap();
		let rel_path =
			if rel_path == "" { file_name.to_string() } else { [&rel_path, file_name].join("/") };

		if entry.is_dir() {
			collect_decls(inputs, &entry, rel_path, provider)?;
		} else if entry.is_file() {
			if !entry.extension().is_some_and(|ext| ext == "stomd") {
				continue;
			}
			// make resolved path, par1/par2/file.stomd -> par1_par2_file
			let mut resolved_path = rel_path.replace('/', "_");
			resolved_path.truncate(rel_path.len() - 6);

			use structom::ImportError::*;
			match provider.load_file(&entry) {
				Ok(decl) => inputs.push(Entry { resolved_path, rel_path, decl }),
				Err(NotFound) => return Err(errors::read_file(&entry.display())(())),
				Err(Parse(err)) => return Err(err.to_string()),
				Err(Other(err)) => {
					return Err(format!("while importing {}: {err}", entry.display()));
				}
			};
		}
	}

	Ok(())
}
