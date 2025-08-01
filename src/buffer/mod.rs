//! Actors that operate on buffers of raw data.

use std::marker::PhantomData;

use crate::{
    actor::Actor,
    buffer::compression::{Compress, Compression, Decompress},
};

pub mod compression;

/// An [`Actor`] that operates on buffers of raw data.
pub trait BufferActor: Actor<Vec<u8>, Vec<u8>> {
    fn compress<T>(self) -> Compression<T, Compress>
    where
        Self: Sized,
        T: compression::Algorithm,
    {
        Compression { mode: Compress, __algorithm: PhantomData }
    }

    fn decompress<T>(self) -> Compression<T, Decompress>
    where
        Self: Sized,
        T: compression::Algorithm,
    {
        Compression { mode: Decompress, __algorithm: PhantomData }
    }
}

impl<A> BufferActor for A where A: Actor<Vec<u8>, Vec<u8>> {}
