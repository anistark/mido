mod document;
mod parse;
mod slug;

pub use document::*;
pub use parse::parse;
pub use slug::{Slugger, slug};
