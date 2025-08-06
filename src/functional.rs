//! Concrete actor implementations for functions and closures.

use std::marker::PhantomData;

use crate::actor::{Actor, State};

/// A concrete wrapper around a [`Functional`].
#[derive(Clone)]
pub struct FunctionalActor<Marker, F>
where
    F: Functional<Marker>,
{
    pub(crate) func: F,
    pub(crate) marker: PhantomData<Marker>,
}

/// A trait implemented by all functions that are actors.
pub trait Functional<Marker>: private::Sealed<Marker> {
    /// The state of this actor.
    type State: Send + Sync + Default;
    /// The input type of this actor.
    type Input: Send;
    /// The output type of this actor.
    type Output: Send;

    /// Execute this actor.
    async fn call(&self, state: State<Self::State>, input: Self::Input) -> Self::Output;
}

/// Internal sealed.
mod private {
    use crate::actor::State;

    pub trait Sealed<Marker> {}

    impl<F, Fut, I, O> Sealed<fn(I) -> Fut> for F
    where
        F: Fn(I) -> Fut,
        Fut: Future<Output = O>,
        I: Send,
        O: Send,
    {
    }

    impl<F, Fut, S, I, O> Sealed<fn(State<S>, I) -> Fut> for F
    where
        F: Fn(State<S>, I) -> Fut,
        Fut: Future<Output = O>,
        S: Send,
        I: Send,
        O: Send,
    {
    }
}

impl<Marker, F> Actor for FunctionalActor<Marker, F>
where
    F: Functional<Marker>,
{
    type State = F::State;
    type Input = F::Input;
    type Output = F::Output;

    fn build(
        self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<F::Input>,
        async_channel::Receiver<F::Output>,
    ) {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();
        let fut = async move {
            let mut state = F::State::default();
            loop {
                let state = State(&mut state);
                let input = in_rx.recv().await.unwrap();
                let output = self.func.call(state, input).await;
                out_tx.send(output).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

impl<F, Fut, I, O> Functional<fn(I) -> Fut> for F
where
    F: Fn(I) -> Fut + Send + private::Sealed<fn(I) -> Fut>,
    Fut: Future<Output = O>,
    I: Send,
    O: Send,
{
    type State = ();
    type Input = I;
    type Output = O;

    async fn call<'a>(&self, _: State<'a, Self::State>, input: Self::Input) -> Self::Output {
        (self)(input).await
    }
}

impl<F, Fut, S, I, O> Functional<fn(State<S>, I) -> Fut> for F
where
    F: Fn(State<S>, I) -> Fut + Send + private::Sealed<fn(State<S>, I) -> Fut>,
    Fut: Future<Output = O>,
    S: Send + Sync + Default,
    I: Send,
    O: Send,
{
    type State = S;
    type Input = I;
    type Output = O;

    async fn call<'a>(&self, state: State<'a, Self::State>, input: Self::Input) -> Self::Output {
        (self)(state, input).await
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::State;
    use crate::functional::Functional;

    async fn enumerate<'a>(mut idx: State<'a, usize>, t: usize) -> (usize, usize) {
        let curr = *idx;
        *idx += 1;
        (curr, t)
    }

    #[tokio::test]
    async fn test_stateful_actor() {
        Functional::call(&enumerate, State(&mut 0), 0)
    }
}
