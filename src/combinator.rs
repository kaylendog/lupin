use std::marker::PhantomData;
use std::time::Duration;

use crate::actor::Actor;
use crate::service::Service;

/// A [`Service`] that attempts to process its inner service indefinitely.
pub struct Repeat<A, I, O> {
    actor: A,
    __marker: PhantomData<(I, O)>,
}

impl<A, I, O> Service<I, O> for Repeat<A, I, O>
where
    A: Actor<I, O>,
    I: Send,
    O: Send,
{
}

/// Accepts an [`Actor`] and produces a [`Service`] that processes the actor indefinitely.
pub fn repeat<A, I, O>(actor: A) -> Repeat<A, I, O>
where
    A: Actor<I, O>,
    I: Send,
    O: Send,
{
    Repeat {
        actor,
        __marker: PhantomData,
    }
}

pub struct Pipe<A, B, I, IO, O> {
    pub(crate) first: A,
    pub(crate) second: B,
    pub(crate) __marker: PhantomData<(I, IO, O)>,
}

impl<A, B, I, IO, O> Service<I, O> for Pipe<A, B, I, IO, O>
where
    A: Service<I, IO>,
    B: Service<IO, O>,
    I: Send,
    IO: Send,
    O: Send,
{
}

pub struct Map<S, F, I, O> {
    pub service: S,
    pub mapper: F,
    pub __marker: PhantomData<(I, O)>,
}

impl<S, F, I, O> Service<I, O> for Map<S, F, I, O> where S: Service<I, O> {}

pub struct Filter<S, F, I, O> {
    pub service: S,
    pub predicate: F,
    pub __marker: PhantomData<(I, O)>,
}

pub struct FanOut<S, B, I, O> {
    pub service: S,
    pub branches: Vec<B>,
    pub __marker: PhantomData<(I, O)>,
}

pub struct FanIn<S, I, O> {
    pub services: Vec<S>,
    pub __marker: PhantomData<(I, O)>,
}

pub struct Retry<S, I, O> {
    pub service: S,
    pub attempts: usize,
    pub __marker: PhantomData<(I, O)>,
}

pub struct Timeout<S, I, O> {
    pub service: S,
    pub duration: Duration,
    pub __marker: PhantomData<(I, O)>,
}

pub struct Buffered<S, I, O> {
    pub service: S,
    pub size: usize,
    pub __marker: PhantomData<(I, O)>,
}

/// Struct for the `parallel` method.
pub struct Parallel<S, I, O> {
    pub service: S,
    pub instances: usize,
    pub __marker: PhantomData<(I, O)>,
}

/// Struct for the `inspect` method.
pub struct Inspect<S, F, I, O> {
    pub service: S,
    pub inspector: F,
    pub __marker: PhantomData<(I, O)>,
}

pub struct Chain<A, B, I, O> {
    pub first: A,
    pub second: B,
    pub __marker: PhantomData<(I, O)>,
}

pub struct Merge<A, B, I, O> {
    pub first: A,
    pub second: B,
    pub __marker: PhantomData<(I, O)>,
}

pub struct WithContext<S, C, I, O> {
    pub service: S,
    pub context: C,
    pub __marker: PhantomData<(I, O)>,
}
