//! Types for [`Service`]s.

use std::marker::PhantomData;

use crate::{actor::Actor, combinator::Pipe};

/// A wrapper around an [`Actor`].
pub trait Service<I, O> {}

/// Extension methods for types implementing [`Service`].
pub trait ServiceExt<I, O>: Service<I, O> {
    fn pipe<B, OB>(self, into: B) -> Pipe<Self, B, I, O, OB>
    where
        Self: Sized,
        B: Service<O, OB>,
    {
        Pipe {
            first: self,
            second: into,
            __marker: PhantomData,
        }
    }
}

impl<S, I, O> ServiceExt<I, O> for S where S: Service<I, O> {}

pub struct Once<A, I, O> {
    actor: A,
    __marker: PhantomData<(I, O)>,
}

impl<A, I, O> Service<I, O> for Once<A, I, O>
where
    A: Actor<I, O>,
    I: Send,
    O: Send,
{
}

pub fn once<A, I, O>(actor: A) -> Once<A, I, O>
where
    A: Actor<I, O>,
    I: Send,
    O: Send,
{
    Once {
        actor,
        __marker: PhantomData,
    }
}

pub struct Source<A, T> {
    actor: A,
    __marker: PhantomData<T>,
}

pub fn source<A, T>(actor: A) -> Source<A, T>
where
    A: Actor<(), T>,
    T: Send,
{
    Source {
        actor,
        __marker: PhantomData,
    }
}

impl<A, T> Service<(), T> for Source<A, T>
where
    A: Actor<(), T>,
    T: Send,
{
}
