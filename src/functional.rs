//! Concrete actor implementations for functions and closures.

use std::marker::PhantomData;

use crate::actor::Actor;

pub struct FunctionalActor<Marker, F>
where
    F: Functional<Marker>,
{
    func: F,
    marker: PhantomData<Marker>,
}

/// A trait implemented by all functions that are actors.
pub trait Functional<Marker>: private::Sealed {
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
    pub trait Sealed {}
}

impl<Marker, F> Actor<F::Input, F::Output> for FunctionalActor<Marker, F>
where
    F: Functional<Marker>,
{
    type State = F::State;

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
    F: Send + private::Sealed,
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
    F: Send + private::Sealed,
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
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };

    async fn enumerate<T: Send>(idx: &mut usize, t: T) -> (usize, T) {
        let curr = *idx;
        *idx += 1;
        (curr, t)
    }

    #[tokio::test]
    async fn test_stateful_actor() {
        let acc = Arc::new(AtomicU64::new(0));
        let count = move |_: ()| {
            let acc = acc.clone();
            async move { acc.fetch_add(1, Ordering::SeqCst) + 1 }
        };

        async fn identity(x: usize) -> usize {
            x
        }

        async fn acc_identity(acc: &mut usize, x: usize) -> usize {
            *acc += x;
            x
        }

        let (task, tx, rx) = count.build();
        tokio::spawn(task);

        tx.send(()).await.unwrap();
        tx.send(()).await.unwrap();
        tx.send(()).await.unwrap();

        assert_eq!(1, rx.recv().await.unwrap());
        assert_eq!(2, rx.recv().await.unwrap());
        assert_eq!(3, rx.recv().await.unwrap());
    }
}
