//! Concrete actor implementations for functions and closures.

use crate::actor::Actor;

impl<F, Fut, I, O> Actor<I, O> for F
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
                out_tx.send((self)(in_rx.recv().await.unwrap()).await).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}
