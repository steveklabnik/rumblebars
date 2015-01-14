#![crate_name="rumblebars"]
#![feature(plugin)]
#![feature(box_syntax)]

#[plugin] extern crate rustlex;
#[plugin] extern crate log;


extern crate "rustc-serialize" as serialize;
extern crate regex;

pub use self::parse::parse;
pub use self::parse::ParseError;
pub use self::parse::Template;
pub use self::parse::HBValHolder;
pub use self::eval::eval;
pub use self::eval::HBData;
pub use self::eval::HBEvalResult;
pub use self::eval::EvalContext;
pub use self::eval::Helper;
pub use self::eval::HelperOptions;
pub use self::eval::HelperOptionsByName;

mod parse;
mod eval;

