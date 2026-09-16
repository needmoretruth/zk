//! Reusable constraint patterns.
//!
//! Every gadget that uses a hint adds the gates that pin the hint's outputs down, so callers never
//! hold an unconstrained wire by accident. Labels passed in become part of the assertion labels,
//! so a failed evaluation names the gadget instance that broke.

mod boolean;
mod merkle;
mod nonzero;
mod range;
mod select;

pub use boolean::assert_boolean;
pub use merkle::{merkle_root, merkle_root_native};
pub use nonzero::assert_nonzero;
pub use range::range_check;
pub use select::select;
