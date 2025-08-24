mod builtins;
mod declaration;
mod errors;
mod parser;
mod value;

pub use declaration::{DeclFile, DeclProvider};
pub use errors::Error;
pub use parser::{ParseOptions, parse, parse_declaration_file, parse_value};
pub use value::{Key, Value};
