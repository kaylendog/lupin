//! [`Actor`] definitions and casting.

use std::marker::PhantomData;

use crate::{
    combinator::{Chunk, Parallel, Pipe},
    functional::{Functional, FunctionalActor},
};

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
    /// This type can be used to store any state required by the actor during its execution.
    type State;

    /// The actors input type.
    type Input;

    /// The actors output type.
    type Output;

    /// Builds the actor into a task and communication channels.
    ///
    /// # Returns
    /// A tuple containing:
    /// - A future representing the actor's main task (drives the actor's logic).
    /// - An [`async_channel::Sender<I>`] for sending input messages to the actor.
    /// - An [`async_channel::Receiver<O>`] for receiving output messages from the actor.
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
    /// - `other`: The next actor in the pipeline, which receives this actor's output as input.
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
    fn parallel(self, n: usize) -> Parallel<Self>
    where
        Self: Sized + Clone,
    {
        Parallel { actor: self, workers: n }
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
    fn chunk(self, size: usize) -> Chunk<Self::IntoActor>
    where
        Self: Sized,
    {
        Chunk { actor: self.into_actor(), size }
    }

    /// See [`Actor::parallel`]
    fn parallel(self, n: usize) -> Parallel<Self::IntoActor>
    where
        Self: Sized + Clone,
    {
        Parallel { actor: self.into_actor(), workers: n }
    }
}

/// A marker type used to distinguish hand-implemented systems from functional systems.
#[doc(hidden)]
#[derive(Clone)]
pub struct IsFunctionalActor;

impl<Marker, F> IntoActor<(IsFunctionalActor, Marker)> for F
where
    Marker: 'static,
    F: Functional<Marker>,
{
    type IntoActor = FunctionalActor<Marker, F>;
    fn into_actor(self) -> Self::IntoActor {
        FunctionalActor { func: self, marker: PhantomData }
    }
}
