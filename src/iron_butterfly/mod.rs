//! Iron Butterfly's *In-A-Gadda-Da-Vida* tone: the pedal (`Patch`), its
//! blocks and its settings (`presets`). Self-contained apart from
//! `crate::pedalkit`, so superPedal carries an identical copy.

mod fuzzrite;
mod organ;
mod patch;
mod presets;

pub use patch::Patch;
