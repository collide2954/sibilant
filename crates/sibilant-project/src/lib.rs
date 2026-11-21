mod autosave;
mod format;
mod location;
mod open;
mod save;
#[cfg(test)]
mod scratch;

pub use autosave::{Autosaved, autosave, restore};
pub use location::{EXTENSION, Location};
pub use open::{OpenError, Opened, open};
pub use save::{Copies, SaveError, save};
