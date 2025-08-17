//! [`Actor`] definitions and casting.

use core::ops::{Deref, DerefMut};

#[cfg(feature = "std")]
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
    #[cfg(feature = "std")]
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
    #[cfg(feature = "std")]
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
    #[cfg(feature = "std")]
    fn chunk(self, size: usize) -> Chunk<Self::IntoActor>
    where
        Self: Sized,
    {
        Chunk { actor: self.into_actor(), size }
    }

    /// See [`Actor::parallel`].
    #[cfg(feature = "std")]
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
