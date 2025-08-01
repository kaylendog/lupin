use crate::actor::{Actor, IntoActor};

/// A concrete implementation of an actor based on a function.
#[derive(Clone)]
pub struct Func<F> {
    f: F,
}

impl<F, Fut, I, O> Actor<I, O> for Func<F>
where
    F: Fn(I) -> Fut,
    Fut: Future<Output = O>,
    I: Send,
    O: Send,
{
    fn build(
        self,
    ) -> (impl Future<Output = ()>, async_channel::Sender<I>, async_channel::Receiver<O>) {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();
        let fut = async move {
            loop {
                out_tx.send((self.f)(in_rx.recv().await.unwrap()).await).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

impl<F, Fut, I, O> IntoActor<I, O> for F
where
    F: Fn(I) -> Fut,
    Fut: Future<Output = O>,
    I: Send,
    O: Send,
{
    type IntoActor = Func<F>;

    fn into_actor(self) -> Self::IntoActor {
        Func { f: self }
    }
}
