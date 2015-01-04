#![crate_name = "mustache"]

#![crate_type = "dylib"]
#![crate_type = "rlib"]

#![feature(phase)]

extern crate "rustc-serialize" as rustc_serialize;

#[phase(plugin, link)]
extern crate log;

pub use builder::{MapBuilder, VecBuilder};
pub use context::Context;
pub use data::Data;
pub use encoder::{Encoder, EncoderResult};
pub use error::Error;
pub use template::Template;

pub mod builder;
mod data;
mod encoder;
mod error;
mod parser;
mod context;
mod compiler;
mod template;

/// Compiles a template from an `Iterator<char>`.
pub fn compile_iter<T: Iterator<char>>(iter: T) -> Template {
    Context::new(Path::new(".")).compile(iter)
}

/// Compiles a template from a path.
/// returns None if the file cannot be read OR the file is not UTF-8 encoded
pub fn compile_path(path: Path) -> Result<Template, Error> {
    Context::new(Path::new(".")).compile_path(path)
}

/// Compiles a template from a string.
pub fn compile_str(template: &str) -> Template {
    compile_iter(template.chars())
}
