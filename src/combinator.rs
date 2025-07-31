use std::{iter::repeat_n, marker::PhantomData};

use futures_concurrency::future::Join;
use futures_lite::FutureExt;
use itertools::Itertools;

use crate::Actor;

#[derive(Clone)]
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

#[derive(Clone)]
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

/// Run `n` instances of the given actor in parallel.
#[derive(Clone)]
pub struct Parallel<A, I, O> {
    pub(crate) actor: A,
    pub(crate) workers: usize,
    pub(crate) __marker: PhantomData<(I, O)>,
}

impl<A, I, O> Actor<I, O> for Parallel<A, I, O>
where
    A: Actor<I, O> + Clone,
    I: Send,
    O: Send,
{
    fn build(
        self,
    ) -> (impl Future<Output = ()>, async_channel::Sender<I>, async_channel::Receiver<O>) {
        let mut worker_futs = Vec::with_capacity(self.workers);
        let mut worker_inputs = Vec::with_capacity(self.workers);
        let mut worker_outputs = Vec::with_capacity(self.workers);

        for (fut, tx, rx) in repeat_n(self.actor, self.workers).map(|actor| actor.build()) {
            worker_futs.push(fut);
            worker_inputs.push(tx);
            worker_outputs.push(rx);
        }

        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();

        let poll_fut = async {
            worker_futs.join().await;
        };

        let input_fut = async move {
            let mut position = 0;
            loop {
                let input = in_rx.recv().await.unwrap();
                let worker = &worker_inputs[position];
                worker.send(input).await.unwrap();
                position = (position + 1) % worker_inputs.len();
            }
        };

        let output_fut = worker_outputs
            .into_iter()
            .map(|rx| {
                let out_tx = out_tx.clone();
                async move {
                    loop {
                        out_tx.send(rx.recv().await.unwrap()).await.unwrap()
                    }
                }
            })
            .collect_vec();
        let output_fut = async {
            output_fut.join().await;
        };

        (poll_fut.or(input_fut).or(output_fut), in_tx, out_rx)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use rand::{Rng, RngCore};

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

    async fn delay(x: usize) -> usize {
        // generate the rng then drop it immediately to avoid holding it across thread boundaries
        let millis = { Duration::from_millis(rand::rng().random_range(0..1000)) };
        tokio::time::sleep(millis).await;
        x
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

    #[tokio::test]
    async fn parallel() {
        let (task, tx, rx) = delay.into_actor().parallel(3).build();
        tokio::spawn(task);

        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        tx.send(3).await.unwrap();

        println!("{}", rx.recv().await.unwrap());
        println!("{}", rx.recv().await.unwrap());
        println!("{}", rx.recv().await.unwrap());
    }
}
