//! Concrete actor implementations for functions and closures.

use std::marker::PhantomData;

use crate::actor::{Actor, IntoActor, State};

/// A concrete wrapper around a [`Functional`].
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

#[cfg(test)]
mod tests {
    use crate::actor::{Actor, IntoActor, State};

    async fn enumerate(mut idx: State<'_, usize>, t: usize) -> (usize, usize) {
        let cur = *idx;
        *idx += 1;
        (cur, t)
    }

    #[tokio::test]
    async fn stateful() {
        let (fut, tx, rx) = enumerate.into_actor().build();
        tokio::task::spawn(fut);
        tx.send(0).await.unwrap();
        tx.send(0).await.unwrap();
        tx.send(0).await.unwrap();

        assert_eq!(0, rx.recv().await.unwrap().0);
        assert_eq!(1, rx.recv().await.unwrap().0);
        assert_eq!(2, rx.recv().await.unwrap().0);
    }
}
