//! A stupendously simple actor framework for the functionally inclined.
//!
//! Lupin is a lightweight actor framework that provides abstractions for
//! building and composing asynchronous systems.
//!
//! An actor is a component that processes input messages and produces output
//! messages asynchronously. This module defines traits and implementations to
//! facilitate the creation and composition of such actors.
//!
//! // Example
//!
//! ```
//! use fenrir::prelude::*;
//!
//! async fn double(x: usize) -> usize {
//!     x * 2
//! }
//!
//! #[tokio::main]
//! async fn main() {}
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

pub mod actor;
pub mod combinator;
pub mod functional;
pub mod numeric;

/// Common types and utilities.
pub mod prelude {
    pub use crate::{actor::*, combinator::pipe};
}
