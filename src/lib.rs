//! A stupendously simple actor framework for the functionally inclined.
//!
//! Fenrir is a lightweight actor framework that provides abstractions for
//! building and composing asynchronous systems.
//!
//! An actor is a component that processes input messages and produces output
//! messages asynchronously. This module defines traits and implementations to
//! facilitate the creation and composition of such actors.

pub mod actor;
pub mod buffer;
pub mod combinator;
pub mod functional;
pub mod numeric;

pub mod prelude {
    pub use crate::actor::*;
    pub use crate::buffer::BufferActor;
    pub use crate::combinator::{chunk, pipe};
}
