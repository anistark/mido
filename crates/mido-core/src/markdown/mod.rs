mod document;
mod parse;
mod slug;

pub use document::*;
pub(crate) use parse::options;
pub use parse::parse;
pub use slug::{Slugger, slug};
