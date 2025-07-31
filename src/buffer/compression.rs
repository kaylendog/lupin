//! Actors for compression algorithms on binary data.

use std::{io::Cursor, marker::PhantomData};

use crate::Actor;

/// Marker struct for compression.
pub struct Compress;

/// Marker struct for decompression.
pub struct Decompress;

/// An algorithm-agnostic compression actor.
pub struct Compression<T, M> {
    pub(crate) mode: M,
    pub(crate) __algorithm: PhantomData<T>,
}

impl<T: Algorithm> Actor<Vec<u8>, Vec<u8>> for Compression<T, Compress> {
    fn build(
        self,
    ) -> (impl Future<Output = ()>, async_channel::Sender<Vec<u8>>, async_channel::Receiver<Vec<u8>>)
    {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();
        let fut = async move {
            loop {
                out_tx.send(T::compress(in_rx.recv().await.unwrap())).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

impl<T: Algorithm> Actor<Vec<u8>, Vec<u8>> for Compression<T, Decompress> {
    fn build(
        self,
    ) -> (impl Future<Output = ()>, async_channel::Sender<Vec<u8>>, async_channel::Receiver<Vec<u8>>)
    {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();
        let fut = async move {
            loop {
                out_tx.send(T::decompress(in_rx.recv().await.unwrap())).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

/// An [`Algorithm`] provided by [`zstd`].
pub struct Zstd;

/// A general-purpose definition of a compression/decompression algorithm.
pub trait Algorithm {
    /// Compress a given buffer.
    fn compress(buf: Vec<u8>) -> Vec<u8>;

    /// Decompress a given buffer.
    fn decompress(buf: Vec<u8>) -> Vec<u8>;
}

impl Algorithm for Zstd {
    fn compress(buf: Vec<u8>) -> Vec<u8> {
        zstd::encode_all(Cursor::new(buf), 0).unwrap()
    }

    fn decompress(buf: Vec<u8>) -> Vec<u8> {
        zstd::decode_all(Cursor::new(buf)).unwrap()
    }
}
