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
pub trait Actor {
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
    fn pipe<T, Marker>(self, other: T) -> Pipe<Self, T::IntoActor>
    where
        Self: Sized,
        T: IntoActor<Marker>,
        T::IntoActor: Actor<Input = Self::Output>,
    {
        Pipe { first: self, second: other.into_actor() }
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
    fn filter<F>(self, predicate: F) -> crate::combinator::Filter<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Output) -> bool + Send + Sync + 'static,
    {
        Filter { actor: self, predicate }
    }
}

/// Conversion trait to turn something into an [`Actor`].
pub trait IntoActor<Marker> {
    /// The type of [`Actor`] that this instance converts into.
    type IntoActor: Actor;

    /// Turns this value into its corresponding [`Actor`].
    fn into_actor(self) -> Self::IntoActor;

    /// See [`Actor::pipe`].
    fn pipe<T, TM>(self, other: T) -> Pipe<Self::IntoActor, T::IntoActor>
    where
        Self: Sized,
        T: IntoActor<TM>,
        T::IntoActor: Actor<Input = <Self::IntoActor as Actor>::Output>,
    {
        Pipe { first: self.into_actor(), second: other.into_actor() }
    }

    /// See [`Actor::chunk`].
    #[cfg(feature = "alloc")]
    fn chunk(self, size: usize) -> Chunk<Self::IntoActor>
    where
        Self: Sized,
    {
        Chunk { actor: self.into_actor(), size }
    }

    /// See [`Actor::parallel`].
    #[cfg(feature = "alloc")]
    fn parallel(self, n: usize) -> Parallel<Self::IntoActor>
    where
        Self: Sized + Clone,
    {
        Parallel { actor: self.into_actor(), workers: n }
    }

    /// See [`Actor::filter`].
    fn filter<F>(self, predicate: F) -> Filter<Self::IntoActor, F>
    where
        Self: Sized,
        F: Fn(&<Self::IntoActor as Actor>::Output) -> bool + Send + Sync + 'static,
    {
        Filter { actor: self.into_actor(), predicate }
    }

    /// See [`Actor::map`].
    fn map<F, U>(self, func: F) -> Map<Self::IntoActor, F>
    where
        Self: Sized,
        F: Fn(<Self::IntoActor as Actor>::Output) -> U + Send + Sync + 'static,
    {
        Map { actor: self.into_actor(), func }
    }

    /// See [`Actor::filter_map`].
    fn filter_map<F, U>(self, func: F) -> FilterMap<Self::IntoActor, F>
    where
        Self: Sized,
        F: Fn(<Self::IntoActor as Actor>::Output) -> Option<U> + Send + Sync + 'static,
    {
        FilterMap { actor: self.into_actor(), func }
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

/// A concrete wrapper around any function that can be converted into an actor.
#[derive(Clone)]
pub struct FunctionalActor<Marker, F> {
    pub(crate) func: F,
    pub(crate) marker: PhantomData<Marker>,
}

impl<F, Fut, I, O> Actor for FunctionalActor<fn(I) -> Fut, F>
where
    F: Fn(I) -> Fut + Send,
    Fut: Future<Output = O>,
    I: Send,
    O: Send,
{
    type State = ();
    type Input = I;
    type Output = O;

    fn build(
        self,
    ) -> (impl Future<Output = ()>, async_channel::Sender<I>, async_channel::Receiver<O>) {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();
        let fut = async move {
            loop {
                let input = in_rx.recv().await.unwrap();
                let output = (self.func)(input).await;
                out_tx.send(output).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

impl<F, S, I, O> Actor for FunctionalActor<fn(&mut S, I) -> O, F>
where
    F: AsyncFn(State<'_, S>, I) -> O,
    S: Send + Sync + Default,
    I: Send + 'static,
    O: Send + 'static,
{
    type State = S;
    type Input = I;
    type Output = O;

    fn build(
        self,
    ) -> (impl Future<Output = ()>, async_channel::Sender<I>, async_channel::Receiver<O>) {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();

        let fut = async move {
            let mut state = S::default();
            loop {
                let input = in_rx.recv().await.unwrap();
                let output = (self.func)(State(&mut state), input).await;
                out_tx.send(output).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

impl<F, Fut, I, O> IntoActor<fn(I) -> Fut> for F
where
    F: Fn(I) -> Fut + Send,
    Fut: Future<Output = O>,
    I: Send,
    O: Send,
{
    type IntoActor = FunctionalActor<fn(I) -> Fut, F>;
    fn into_actor(self) -> Self::IntoActor {
        FunctionalActor { func: self, marker: PhantomData }
    }
}

impl<F, S, I, O> IntoActor<fn(&mut S, I) -> O> for F
where
    F: AsyncFn(State<'_, S>, I) -> O,
    S: Send + Sync + Default + 'static,
    I: Send + 'static,
    O: Send + 'static,
{
    type IntoActor = FunctionalActor<fn(&mut S, I) -> O, F>;
    fn into_actor(self) -> Self::IntoActor {
        FunctionalActor { func: self, marker: PhantomData }
    }
}

/// A wrapper type that turns a [`Stream`] into an actor.
pub struct StreamActor<S> {
    inner: S,
}

/// Marker type for stream-based actors.
struct IsStream;

impl<S> Actor for StreamActor<S>
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
                let Some(next) = self.inner.next().await else {
                    break;
                };
                out_tx.send(next).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

impl<S> IntoActor<IsStream> for S
where
    S: Stream + Unpin,
{
    type IntoActor = StreamActor<S>;

    fn into_actor(self) -> Self::IntoActor {
        StreamActor { inner: self }
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::{Actor, IntoActor, State};

    async fn enumerate(mut idx: State<'_, usize>, t: usize) -> (usize, usize) {
        let cur = *idx;
        *idx += 1;
        (cur, t)
    }

    #[tokio::test]
    async fn test_stateful() {
        let (fut, tx, rx) = enumerate.into_actor().build();
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
        let stream_actor = test_stream.into_actor();

        let (fut, _in_tx, out_rx) = stream_actor.build();

        tokio::spawn(fut);

        assert_eq!(out_rx.recv().await.unwrap(), 1);
        assert_eq!(out_rx.recv().await.unwrap(), 2);
        assert_eq!(out_rx.recv().await.unwrap(), 3);
        assert!(out_rx.try_recv().is_err());
    }
}
