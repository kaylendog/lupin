use std::marker::PhantomData;

use futures_lite::FutureExt;

use crate::Actor;

pub struct Pipe<A, B, I, IO, O> {
    pub(crate) first: A,
    pub(crate) second: B,
    pub(crate) __marker: PhantomData<(I, IO, O)>,
}

impl<A, B, I, IO, O> Actor<I, O> for Pipe<A, B, I, IO, O>
where
    A: Actor<I, IO>,
    B: Actor<IO, O>,
    I: Send,
    IO: Send,
    O: Send,
{
    fn build(
        self,
    ) -> (impl Future<Output = ()>, async_channel::Sender<I>, async_channel::Receiver<O>) {
        let (first_fut, first_in, first_out) = self.first.build();
        let (second_fut, second_in, second_out) = self.second.build();
        let fut = async move {
            loop {
                second_in.send(first_out.recv().await.unwrap()).await.unwrap();
            }
        };
        (first_fut.or(second_fut).or(fut), first_in, second_out)
    }
}

pub struct Chunk<A, I, O> {
    pub(crate) actor: A,
    pub(crate) size: usize,
    pub(crate) __marker: PhantomData<(I, O)>,
}

impl<A, I, O> Actor<I, Vec<O>> for Chunk<A, I, O>
where
    A: Actor<I, O>,
    I: Send,
    O: Send,
{
    fn build(
        self,
    ) -> (impl Future<Output = ()>, async_channel::Sender<I>, async_channel::Receiver<Vec<O>>) {
        let (child_fut, child_tx, child_rx) = self.actor.build();
        let (tx, rx) = async_channel::unbounded();
        let fut = async move {
            let mut buffer = Vec::with_capacity(self.size);
            loop {
                // fill buffer to capacity
                buffer.push(child_rx.recv().await.unwrap());
                if buffer.len() < self.size {
                    continue;
                }
                // send chunk
                tx.send(buffer).await.unwrap();
                buffer = Vec::with_capacity(self.size);
            }
        };
        (child_fut.or(fut), child_tx, rx)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Actor, IntoActor};

    async fn identity(x: usize) -> usize {
        x
    }

    async fn add1(x: usize) -> usize {
        x + 1
    }

    async fn mul2(x: usize) -> usize {
        x * 2
    }

    async fn sum(items: Vec<usize>) -> usize {
        items.iter().sum()
    }

    #[tokio::test]
    async fn pipe() {
        let (task, tx, rx) = add1.into_actor().pipe(mul2.into_actor()).build();
        tokio::spawn(task);
        tx.send(1).await.unwrap();
        assert_eq!(4, rx.recv().await.unwrap());
    }

    #[tokio::test]
    async fn chunk() {
        let (task, tx, rx) = identity.into_actor().chunk(3).build();
        tokio::spawn(task);
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        tx.send(3).await.unwrap();
        assert_eq!(vec![1, 2, 3], rx.recv().await.unwrap());

        // with sum
        let (task, tx, rx) = identity.into_actor().chunk(3).pipe(sum.into_actor()).build();
        tokio::spawn(task);
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        tx.send(3).await.unwrap();
        assert_eq!(6, rx.recv().await.unwrap());
    }
}
