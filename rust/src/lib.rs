mod declaration;
mod errors;
mod parser;
mod value;

pub use errors::Error;
pub use parser::parse;
pub use value::{Key, Value};
