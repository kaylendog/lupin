//! A stupendously simple actor framework for the functionally inclined.
//!
//! Lupin is a lightweight actor framework that provides abstractions for
//! building and composing asynchronous systems.
//!
//! An actor is a component that processes input messages and produces output
//! messages asynchronously. This module defines traits and implementations to
//! facilitate the creation and composition of such actors.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod actor;
pub mod combinator;
// pub mod numeric;

/// Common types and utilities.
pub mod prelude {
    #[cfg(feature = "alloc")]
    pub use crate::combinator::{chunk, parallel};
    pub use crate::{
        actor::Actor,
        combinator::{filter, filter_map, map, pipe},
    };
}
