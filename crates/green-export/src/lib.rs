//! Green's export: document blocks to Typst source, Typst to PDF, and the PDF
//! audited against the limits of design §7.

pub mod audit;
pub mod compile;
pub mod doc;
pub mod export;
pub mod glyphs;
pub mod typst;

pub use audit::{Audit, Font, Warning};
pub use compile::{Cli, Compiler};
pub use doc::{Align, Block, Inline, Item, Style};
pub use export::{Export, export};
pub use glyphs::Missing;

#[cfg(test)]
mod tests;
