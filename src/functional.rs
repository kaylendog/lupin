//! Concrete actor implementations for functions and closures.

use std::marker::PhantomData;

use crate::actor::Actor;

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
    type State: Send + Default;
    /// The input type of this actor.
    type Input: Send;
    /// The output type of this actor.
    type Output: Send;

    /// Execute this actor.
    async fn call(&mut self, state: &mut Self::State, input: Self::Input) -> Self::Output;
}

/// Internal sealed.
mod private {
    pub trait Sealed<Marker> {}

    impl<F, Fut, I, O> Sealed<fn(I) -> Fut> for F
    where
        for<'a> &'a mut F: FnMut(I) -> Fut,
        Fut: Future<Output = O>,
        I: Send,
        O: Send,
    {
    }

    impl<F, Fut, S, I, O> Sealed<fn(&mut S, I) -> Fut> for F
    where
        for<'a> &'a mut F: FnMut(I) -> Fut,
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
        mut self,
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
                let input = in_rx.recv().await.unwrap();
                let output = self.func.call(&mut state, input).await;
                out_tx.send(output).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

impl<F, Fut, I, O> Functional<fn(I) -> Fut> for F
where
    F: Send + private::Sealed<fn(I) -> Fut>,
    for<'a> &'a mut F: FnMut(I) -> Fut,
    Fut: Future<Output = O>,
    I: Send,
    O: Send,
{
    type State = ();
    type Input = I;
    type Output = O;

    async fn call(&mut self, _: &mut Self::State, input: Self::Input) -> Self::Output {
        // Rustc fails to recognise that `self` is a fucntion, so we use this inner method.
        async fn call_inner<Fut, I, O>(mut f: impl FnMut(I) -> Fut, input: I) -> O
        where
            Fut: Future<Output = O>,
        {
            f(input).await
        }
        call_inner(self, input).await
    }
}

impl<F, Fut, S, I, O> Functional<fn(&mut S, I) -> Fut> for F
where
    F: Send + private::Sealed<fn(&mut S, I) -> Fut>,
    for<'a> &'a mut F: FnMut(&mut S, I) -> Fut,
    Fut: Future<Output = O>,
    S: Send + Default,
    I: Send,
    O: Send,
{
    type State = S;
    type Input = I;
    type Output = O;

    async fn call(&mut self, state: &mut Self::State, input: Self::Input) -> Self::Output {
        // Rustc fails to recognise that `self` is a fucntion, so we use this inner method.
        async fn call_inner<Fut, S, I, O>(
            mut f: impl FnMut(&mut S, I) -> Fut,
            state: &mut S,
            input: I,
        ) -> O
        where
            Fut: Future<Output = O>,
        {
            f(state, input).await
        }
        call_inner(self, state, input).await
    }
}

#[cfg(test)]
mod tests {

    use crate::functional::Functional;

    async fn enumerate(idx: &mut usize, t: usize) -> (usize, usize) {
        let curr = *idx;
        *idx += 1;
        (curr, t)
    }

    #[tokio::test]
    async fn test_stateful_actor() {
        let (task, tx, rx) = Functional::call(&mut enumerate, &mut 0, 0);
        tokio::spawn(task);

        tx.send(()).await.unwrap();
        tx.send(()).await.unwrap();
        tx.send(()).await.unwrap();

        assert_eq!(1, rx.recv().await.unwrap());
        assert_eq!(2, rx.recv().await.unwrap());
        assert_eq!(3, rx.recv().await.unwrap());
    }
}
