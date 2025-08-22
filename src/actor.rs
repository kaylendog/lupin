//! [`Actor`] definitions and casting.

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use futures_lite::{Stream, StreamExt};

#[cfg(feature = "alloc")]
use crate::combinator::{Chunk, Parallel};
use crate::combinator::{Filter, FilterMap, Map, Pipe};

/// A trait representing an asynchronous actor.
///
/// An actor processes input messages and produces output messages
/// asynchronously. Actors can be composed into pipelines for more complex
/// workflows.
///
/// # Type Parameters
/// - `I`: The input message type. Must implement [`Send`].
/// - `O`: The output message type. Must implement [`Send`].
pub trait Actor<Marker> {
    /// The actor's internal state.
    ///
    /// This type can be used to store any state required by the actor during
    /// its execution.
    type State;

    /// The actors input type.
    type Input;

    /// The actors output type.
    type Output;

    /// Builds the actor into a task and communication channels.
    ///
    /// # Returns
    /// A tuple containing:
    /// - A future representing the actor's main task (drives the actor's
    ///   logic).
    /// - An [`async_channel::Sender<I>`] for sending input messages to the
    ///   actor.
    /// - An [`async_channel::Receiver<O>`] for receiving output messages from
    ///   the actor.
    ///
    /// # Example
    /// ```rust,ignore
    /// let (task, input, output) = my_actor.build();
    /// ```
    fn build(
        self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<Self::Input>,
        async_channel::Receiver<Self::Output>,
    );

    /// Composes this actor with another actor to form a pipeline.
    ///
    /// # Arguments
    /// - `other`: The next actor in the pipeline, which receives this actor's
    ///   output as input.
    ///
    /// # Returns
    /// A [`Pipe`] representing the composed pipeline of two actors.
    ///
    /// # Example
    /// ```rust,ignore
    /// let pipeline = actor1.pipe(actor2);
    /// ```
    fn pipe<MT, T>(self, other: T) -> Pipe<Self, T>
    where
        Self: Sized,
        T: Actor<MT, Input = Self::Output>,
    {
        Pipe { first: self, second: other }
    }

    /// Groups up to `size` results from this actor into chunks.
    ///
    /// # Arguments
    /// - `size`: The maximum number of items per chunk.
    ///
    /// # Returns
    /// A [`Chunk`] combinator that batches output messages.
    ///
    /// # Example
    /// ```rust,ignore
    /// let chunked = actor.chunk(10);
    /// ```
    #[cfg(feature = "alloc")]
    fn chunk(self, size: usize) -> Chunk<Self>
    where
        Self: Sized,
    {
        Chunk { actor: self, size }
    }

    /// Runs this actor in parallel using `n` worker tasks.
    ///
    /// # Arguments
    /// - `n`: The number of parallel workers to spawn.
    ///
    /// # Returns
    /// A [`Parallel`] combinator that distributes input messages among workers.
    ///
    /// # Example
    /// ```rust,ignore
    /// let parallel = actor.parallel(4);
    /// ```
    #[cfg(feature = "alloc")]
    fn parallel(self, n: usize) -> Parallel<Self>
    where
        Self: Sized + Clone,
    {
        Parallel { actor: self, workers: n }
    }

    /// Filters output messages from this actor using a predicate.
    ///
    /// # Arguments
    /// - `predicate`: A function or closure that takes a reference to an output
    ///   message and returns `true` to keep the message, or `false` to discard
    ///   it.
    ///
    /// # Returns
    /// A [`crate::combinator::Filter`] combinator that only passes through
    /// output messages matching the predicate.
    ///
    /// # Example
    /// ```rust,ignore
    /// let filtered = actor.filter(|msg| msg.is_valid());
    /// ```
    fn filter<F>(self, predicate: F) -> Filter<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Output) -> bool + Send + Sync + 'static,
    {
        Filter { actor: self, predicate }
    }

    /// Maps output messages from this actor using a function.
    ///
    /// # Arguments
    /// - `f`: A function or closure that takes an output message and returns a new value.
    ///
    /// # Returns
    /// A [`crate::combinator::Map`] combinator that transforms output messages.
    ///
    /// # Example
    /// ```rust,ignore
    /// let mapped = actor.map(|msg| msg.to_string());
    /// ```
    fn map<F, T>(self, func: F) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Output) -> T + Send + Sync + 'static,
    {
        Map { actor: self, func }
    }

    /// Applies a function that may return `Some` or `None` to each output message.
    ///
    /// # Arguments
    /// - `f`: A function or closure that takes an output message and returns an `Option<T>`.
    ///
    /// # Returns
    /// A [`crate::combinator::FilterMap`] combinator that transforms and filters output messages.
    ///
    /// # Example
    /// ```rust,ignore
    /// let filtered_mapped = actor.filter_map(|msg| if msg.is_valid() { Some(msg.value()) } else { None });
    /// ```
    fn filter_map<F, T>(self, func: F) -> FilterMap<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Output) -> Option<T> + Send + Sync + 'static,
    {
        FilterMap { actor: self, func }
    }
}

/// A wrapper type that denotes mutable state.
#[derive(Debug)]
pub struct State<'a, T: ?Sized>(pub(crate) &'a mut T);

impl<'i, T: ?Sized> Deref for State<'i, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'i, T: ?Sized> DerefMut for State<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<F, I, O> Actor<fn(I) -> O> for F
where
    F: AsyncFn(I) -> O,
{
    type State = ();
    type Input = I;
    type Output = O;

    fn build(
        self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<Self::Input>,
        async_channel::Receiver<Self::Output>,
    ) {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();
        let fut = async move {
            loop {
                let input = in_rx.recv().await.unwrap();
                let output = (self)(input).await;
                out_tx.send(output).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

impl<F, S, I, O> Actor<fn(&mut S, I) -> O> for F
where
    F: AsyncFn(&mut S, I) -> O,
    S: Default,
{
    type State = S;
    type Input = I;
    type Output = O;

    fn build(
        self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<Self::Input>,
        async_channel::Receiver<Self::Output>,
    ) {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();

        let fut = async move {
            let mut state = S::default();
            loop {
                let input = in_rx.recv().await.unwrap();
                let output = (self)(&mut state, input).await;
                out_tx.send(output).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

/// Marker type for stream-based actors.
struct IsStream<S> {
    __marker: PhantomData<S>,
}

impl<S> Actor<IsStream<S>> for S
where
    S: Stream + Unpin,
{
    type State = ();
    type Input = ();
    type Output = S::Item;

    fn build(
        mut self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<Self::Input>,
        async_channel::Receiver<Self::Output>,
    ) {
        let (in_tx, _) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();

        let fut = async move {
            loop {
                let Some(next) = self.next().await else {
                    break;
                };
                out_tx.send(next).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::Actor;

    async fn enumerate(idx: &mut usize, t: usize) -> (usize, usize) {
        let cur = *idx;
        *idx += 1;
        (cur, t)
    }

    #[tokio::test]
    async fn test_stateful() {
        let (fut, tx, rx) = enumerate.build();
        tokio::task::spawn(fut);
        tx.send(0).await.unwrap();
        tx.send(0).await.unwrap();
        tx.send(0).await.unwrap();

        assert_eq!(0, rx.recv().await.unwrap().0);
        assert_eq!(1, rx.recv().await.unwrap().0);
        assert_eq!(2, rx.recv().await.unwrap().0);
    }

    #[cfg(feature = "alloc")]
    #[tokio::test]
    async fn test_stream_iter() {
        use alloc::vec;

        let test_stream = futures_lite::stream::iter(vec![1, 2, 3]);

        let (fut, _in_tx, out_rx) = Actor::build(test_stream);

        tokio::spawn(fut);

        assert_eq!(out_rx.recv().await.unwrap(), 1);
        assert_eq!(out_rx.recv().await.unwrap(), 2);
        assert_eq!(out_rx.recv().await.unwrap(), 3);
        assert!(out_rx.try_recv().is_err());
    }
}
