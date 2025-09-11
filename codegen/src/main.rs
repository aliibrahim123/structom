mod rust;
pub mod utils;

use std::fs::{canonicalize, create_dir, create_dir_all, remove_dir_all};
use std::{fs::read_dir, path::Path};

use clap::{Parser, ValueEnum};
use structom::FSProvider;
use structom::{DeclFile, LoadFileError};

use crate::rust::to_rust;

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Lang {
	Rust,
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

fn main() -> Result<(), String> {
	let Args { input, output, lang } = Args::parse();

	// prepare output directory
	create_dir_all(&output).map_err(|_| format!("unable to create directory \"{output}\""))?;
	let out_path =
		canonicalize(&output).map_err(|_| format!("unable to read directory \"{output}\""))?;
	remove_dir_all(&out_path).map_err(|_| format!("unable to remove directory \"{output}\""))?;
	create_dir(&out_path).map_err(|_| format!("unable to create directory \"{output}\""))?;

	// read declerations
	let input =
		canonicalize(&input).map_err(|_| format!("unable to read directory \"{input}\""))?;
	let provider = FSProvider::new(&input).unwrap();
	let mut inputs = Vec::new();
	walk_fs(&mut inputs, &input, "".to_string(), &provider)?;

	// generate code
	match lang {
		Lang::Rust => to_rust(&inputs, input.to_str().unwrap(), &out_path, &provider)?,
	}

	Ok(())
}

/// file input
#[derive(Debug)]
pub struct Entry<'a> {
	/// relative path in the input directory
	pub rel_path: String,
	/// relative path in the output directory
	pub resolved_path: String,
	pub decl: &'a DeclFile,
}

/// recursively walk the declerations directory and collect declerations
pub fn walk_fs<'a>(
	inputs: &mut Vec<Entry<'a>>, path: &Path, rel_path: String, provider: &'a FSProvider,
) -> Result<(), String> {
	// every entry in the directory
	for entry in read_dir(path).map_err(|_| format!("unable to read directory \"{path:?}\""))? {
		let entry = entry.map_err(|_| format!("unable to read directory \"{path:?}\""))?.path();
		// resolve relative path
		let file_name = entry.file_name().unwrap().to_str().unwrap();
		let rel_path =
			if rel_path == "" { file_name.to_string() } else { [&rel_path, file_name].join("/") };

		if entry.is_dir() {
			walk_fs(inputs, &entry, rel_path, provider)?;
		} else if entry.is_file() {
			// skip non decleration files
			if !entry.extension().is_some_and(|ext| ext == "stomd") {
				continue;
			}
			// make resolved path, par1/par2/file.stomd -> par1_par2_file
			let mut resolved_path = rel_path.replace('/', "_");
			resolved_path.truncate(rel_path.len() - 6);

			// parse file and redirect errors
			match provider.load_file(&entry) {
				Ok(decl) => inputs.push(Entry { resolved_path, rel_path, decl }),
				Err(LoadFileError::IO(_)) => {
					return Err(format!("unable to read file \"{}\"", entry.display()));
				}
				Err(LoadFileError::Parse(structom::ParserError::TypeError(err))) => {
					return Err(err);
				}
				Err(LoadFileError::Parse(structom::ParserError::SyntaxError(err))) => {
					return Err(format!("{err} at decleration file \"{}\"", entry.display()));
				}
			};
		}
	}

	Ok(())
}
