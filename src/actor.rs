//! [`Actor`] definitions and casting.

use core::marker::PhantomData;

use futures_lite::StreamExt;

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
    type State: Send + Sync + Default;

    /// The actors input type.
    type Input: Send;

    /// The actors output type.
    type Output: Send;

    /// Builds the actor into a task and communication channels.
    ///
    /// # Arguments
    /// - `initial_state`: The initial state for the actor.
    ///
    /// # Returns
    /// A tuple containing:
    /// - A future representing the actor's main task, which drives the actor's
    ///   logic.
    /// - An [`ActorRef<Self::Input, Self::Output>`] for interacting with the
    ///   actor.
    ///
    /// # Example
    /// ```rust,ignore
    /// let (task, actor_ref) = my_actor.build(initial_state);
    /// ```
    fn build(self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>);

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
    /// - `f`: A function or closure that takes an output message and returns a
    ///   new value.
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

    /// Applies a function that may return `Some` or `None` to each output
    /// message.
    ///
    /// # Arguments
    /// - `f`: A function or closure that takes an output message and returns an
    ///   `Option<T>`.
    ///
    /// # Returns
    /// A [`crate::combinator::FilterMap`] combinator that transforms and
    /// filters output messages.
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

/// A marker struct for indicating Sink actors.
pub struct Sink<T> {
    __marker: PhantomData<T>,
}

/// A marker struct for indicating Source actors.
pub struct Source<T> {
    __marker: PhantomData<T>,
}

/// A marker struct for indicating Pipeline actors.
pub struct Pipeline<T> {
    __marker: PhantomData<T>,
}

/// A marker struct for indicating Stream actors.
pub struct Stream<T> {
    __marker: PhantomData<T>,
}

impl<F, O> Actor<Source<fn() -> O>> for F
where
    F: AsyncFn() -> O,
    O: Send,
{
    type State = ();
    type Input = ();
    type Output = O;

    fn build(self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>) {
        let (out_tx, out_rx) = async_channel::bounded(1);
        let fut = async move {
            let output = self().await;
            while out_tx.is_full() {
                futures_lite::future::yield_now().await;
            }
            out_tx.send(output).await.unwrap();
        };
        (fut, ActorRef::Source(out_rx))
    }
}

impl<F, S, O> Actor<Source<fn(&mut S) -> O>> for F
where
    F: AsyncFn(&mut S) -> O,
    S: Send + Sync + Default,
    O: Send,
{
    type State = S;
    type Input = ();
    type Output = O;

    fn build(self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>) {
        let (out_tx, out_rx) = async_channel::bounded(1);
        let fut = async move {
            let mut state = S::default();
            loop {
                let output = self(&mut state).await;
                while out_tx.is_full() {
                    futures_lite::future::yield_now().await;
                }
                out_tx.send(output).await.unwrap();
            }
        };
        (fut, ActorRef::Source(out_rx))
    }
}

impl<F, I, O> Actor<Pipeline<fn(&I) -> O>> for F
where
    F: AsyncFn(&I) -> O,
    I: Send,
    O: Send,
{
    type State = ();
    type Input = I;
    type Output = O;

    fn build(self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>) {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();
        let fut = async move {
            loop {
                let input = in_rx.recv().await.unwrap();
                let output = (self)(&input).await;
                out_tx.send(output).await.unwrap();
            }
        };
        (fut, ActorRef::Pipeline(in_tx, out_rx))
    }
}

impl<F, S, I, O> Actor<Pipeline<fn(&mut S, &I) -> O>> for F
where
    F: AsyncFn(&mut S, &I) -> O,
    S: Send + Sync + Default,
    I: Send,
    O: Send,
{
    type State = S;
    type Input = I;
    type Output = O;

    fn build(self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>) {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();
        let fut = async move {
            let mut state = S::default();
            loop {
                let input = in_rx.recv().await.unwrap();
                let output = self(&mut state, &input).await;
                out_tx.send(output).await.unwrap();
            }
        };
        (fut, ActorRef::Pipeline(in_tx, out_rx))
    }
}

impl<S> Actor<Stream<S>> for S
where
    S: futures_lite::Stream + Unpin,
    S::Item: core::marker::Send,
{
    type State = ();
    type Input = ();
    type Output = S::Item;

    fn build(mut self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>) {
        let (out_tx, out_rx) = async_channel::unbounded();
        let fut = async move {
            loop {
                let Some(next) = self.next().await else {
                    break;
                };
                while out_tx.is_full() {
                    futures_lite::future::yield_now().await;
                }
                out_tx.send(next).await.unwrap();
            }
        };
        (fut, ActorRef::Source(out_rx))
    }
}

/// An enumeration representing references to different types of actors.
pub enum ActorRef<I, O> {
    /// Represents a closed actor.
    Closed,
    /// Represents a source actor with a receiver for output items.
    Source(async_channel::Receiver<O>),
    /// Represents a pipeline actor with both a sender for input items and a
    /// receiver for output items.
    Pipeline(async_channel::Sender<I>, async_channel::Receiver<O>),
}

impl<I, O> ActorRef<I, O> {
    /// Joins two actor references, creating a new pipeline.
    ///
    /// # Arguments
    /// - `self`: The first actor reference.
    /// - `other`: The second actor reference, which receives the output of the
    ///   first.
    ///
    /// # Returns
    /// A tuple containing:
    /// - A new [`ActorRef`] that represents the joined pipeline.
    /// - An [`async_channel::Receiver`] for the intermediate output messages.
    /// - An [`async_channel::Sender`] for the intermediate input messages.
    ///
    /// # Example
    /// ```rust,ignore
    /// let (joined_ref, intermediate_rx, intermediate_tx) = actor_ref1.join(actor_ref2);
    /// ```
    pub fn pipe<T>(
        self,
        other: ActorRef<O, T>,
    ) -> (ActorRef<I, T>, async_channel::Receiver<O>, async_channel::Sender<O>) {
        match (self, other) {
            (ActorRef::Source(rx), ActorRef::Pipeline(tx, out)) => (ActorRef::Source(out), rx, tx),
            (ActorRef::Pipeline(a_tx, a_rx), ActorRef::Pipeline(b_tx, b_rx)) => {
                (ActorRef::Pipeline(a_tx, b_rx), a_rx, b_tx)
            }
            _ => unreachable!(),
        }
    }

    /// Splits the current actor reference into separate components for further
    /// composition.
    ///
    /// # Returns
    /// A tuple containing:
    /// - A new [`ActorRef`] with the same input type but a new output type.
    /// - An [`async_channel::Receiver`] for receiving the current output
    ///   messages.
    /// - An [`async_channel::Sender`] for sending messages of the new output
    ///   type.
    ///
    /// # Example
    /// ```rust,ignore
    /// let (new_actor_ref, current_rx, new_tx) = actor_ref.splice();
    /// ```
    pub fn splice<T>(
        self,
    ) -> (ActorRef<I, T>, async_channel::Receiver<O>, async_channel::Sender<T>) {
        let (tx, rx) = async_channel::unbounded();
        match self {
            ActorRef::Closed => panic!(),
            ActorRef::Source(receiver) => (ActorRef::Source(rx), receiver, tx),
            ActorRef::Pipeline(sender, receiver) => (ActorRef::Pipeline(sender, rx), receiver, tx),
        }
    }

    /// Send this actor some data.
    pub async fn send(&self, item: I) -> Result<(), async_channel::SendError<I>> {
        match &self {
            ActorRef::Pipeline(tx, _) => tx.send(item).await,
            _ => unreachable!(),
        }
    }

    /// Attempt to pull some data from this actor.
    pub async fn recv(&self) -> Result<O, async_channel::RecvError> {
        match &self {
            ActorRef::Source(rx) => rx.recv().await,
            ActorRef::Pipeline(_, rx) => rx.recv().await,
            _ => unreachable!(),
        }
    }
}

impl<I, O> Clone for ActorRef<I, O> {
    fn clone(&self) -> Self {
        match self {
            Self::Closed => Self::Closed,
            Self::Source(rx) => Self::Source(rx.clone()),
            Self::Pipeline(tx, rx) => Self::Pipeline(tx.clone(), rx.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::Actor;

    async fn source() -> usize {
        0
    }

    async fn source_stateful(state: &mut usize) -> usize {
        let x = *state;
        *state += 1;
        x
    }

    async fn sink(_input: &usize) {}

    async fn sink_stateful(state: &mut usize, _input: &usize) {
        *state += 1;
    }

    async fn pipeline(input: &usize) -> usize {
        input + 1
    }

    async fn pipeline_stateful(acc: &mut usize, input: &usize) -> usize {
        *acc += *input;
        *acc
    }

    #[tokio::test]
    async fn test_source() {
        let (fut, actor) = source.build();
        tokio::spawn(fut);
        assert_eq!(0, actor.recv().await.unwrap());
    }

    #[tokio::test]
    async fn test_source_stateful() {
        let (fut, actor) = source_stateful.build();
        tokio::spawn(fut);
        assert_eq!(0, actor.recv().await.unwrap());
        assert_eq!(1, actor.recv().await.unwrap());
    }

    #[tokio::test]
    async fn test_sink() {
        let (fut, actor) = sink.build();
        tokio::spawn(fut);
        actor.send(0).await.unwrap();
    }

    #[tokio::test]
    async fn test_sink_stateful() {
        let (fut, actor) = sink_stateful.build();
        tokio::spawn(fut);
        actor.send(0).await.unwrap();
    }

    #[tokio::test]
    async fn test_pipeline() {
        let (fut, actor) = pipeline.build();
        tokio::spawn(fut);
        actor.send(0).await.unwrap();
        assert_eq!(1, actor.recv().await.unwrap());
        actor.send(0).await.unwrap();
        assert_eq!(1, actor.recv().await.unwrap());
    }

    #[tokio::test]
    async fn test_pipeline_stateful() {
        let (fut, actor) = pipeline_stateful.build();
        tokio::spawn(fut);
        actor.send(1).await.unwrap();
        assert_eq!(1, actor.recv().await.unwrap());
        actor.send(1).await.unwrap();
        assert_eq!(2, actor.recv().await.unwrap());
    }
}
