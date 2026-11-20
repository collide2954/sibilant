mod location;
mod save;
#[cfg(test)]
mod scratch;

pub use location::{EXTENSION, Location};
pub use save::SaveError;
