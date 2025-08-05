//! Concrete actor implementations for functions and closures.

use crate::actor::Actor;

impl<F, Fut, I, O> Actor<I, O> for F
where
    F: Fn(I) -> Fut,
    Fut: Future<Output = O>,
    I: Send,
    O: Send,
{
    type State = ();

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

impl<Fut, S, I, O> Actor<I, O> for fn(&mut S, I) -> Fut
where
    Fut: Future<Output = O>,
    S: Default,
    I: Send,
    O: Send,
{
    type State = S;

    fn build(
        self,
    ) -> (impl Future<Output = ()>, async_channel::Sender<I>, async_channel::Receiver<O>) {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();
        let fut = async move {
            let mut state = S::default();
            loop {
                out_tx.send((self)(&mut state, in_rx.recv().await.unwrap()).await).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };

    use crate::actor::Actor;

    async fn enumerate<T>(idx: &mut usize, t: T) -> (usize, T) {
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

        enumerate.build();

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
