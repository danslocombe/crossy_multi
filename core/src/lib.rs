#![allow(unused_parens)]

#[macro_use]
extern crate num_derive;

pub mod game;
pub mod interop;
pub mod client;

use num_traits::FromPrimitive;
pub use game::*;