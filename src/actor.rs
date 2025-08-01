//! [`Actor`] definitions and casting.

use std::marker::PhantomData;

use crate::combinator::{Chunk, Parallel, Pipe};

/// A trait representing an asynchronous actor.
///
/// An actor processes input messages and produces output messages
/// asynchronously. Actors can be composed into pipelines for more complex
/// workflows.
pub trait Actor<I, O>
where
    I: Send,
    O: Send,
{
    /// Builds the actor into a task and communication channels.
    ///
    /// # Returns
    /// A tuple containing:
    /// - A future representing the actor's task.
    /// - A sender for input messages.
    /// - A receiver for output messages.
    fn build(
        self,
    ) -> (impl Future<Output = ()>, async_channel::Sender<I>, async_channel::Receiver<O>);

    /// Composes this actor with another actor to form a pipeline.
    ///
    /// # Arguments
    /// - `other`: The next actor in the pipeline.
    ///
    /// # Returns
    /// A [`Pipe`] representing the composed pipeline.
    fn pipe<B, OB>(self, other: B) -> Pipe<Self, B, O>
    where
        Self: Sized,
        B: Actor<O, OB>,
        OB: Send,
    {
        Pipe { first: self, second: other, __marker: PhantomData }
    }

    /// Chunk up to `size` results from this actor.
    fn chunk(self, size: usize) -> Chunk<Self>
    where
        Self: Sized,
    {
        Chunk { actor: self, size }
    }

    /// Run this actor in parallel using `n` workers.
    fn parallel(self, n: usize) -> Parallel<Self>
    where
        Self: Sized + Clone,
    {
        Parallel { actor: self, workers: n }
    }
}

/// A trait for converting an object into an actor.
///
/// This trait is useful for adapting functions or other objects into actors.
pub trait IntoActor<I, O>
where
    I: Send,
    O: Send,
{
    /// The type of actor produced by this conversion.
    type IntoActor: Actor<I, O>;

    /// Converts the object into an actor.
    fn into_actor(self) -> Self::IntoActor;
}
