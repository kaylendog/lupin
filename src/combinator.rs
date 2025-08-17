//! Combinatorial actors that enable larger systems to be built by connecting
//! actors together.

#[cfg(feature = "std")]
use core::iter::repeat_n;

#[cfg(feature = "std")]
use futures_concurrency::future::Join;
use futures_lite::FutureExt;
#[cfg(feature = "std")]
use itertools::Itertools;

use crate::actor::{Actor, IntoActor};

/// Pipes the output of one actor into another.
///
/// This combinator allows you to connect two actors such that the output of the
/// first is sent as input to the second. The `Pipe` struct holds both actors
/// internally.
#[derive(Clone)]
pub struct Pipe<A, B>
where
    A: Actor,
    B: Actor<Input = A::Output>,
{
    pub(crate) first: A,
    pub(crate) second: B,
}

impl<A, B> Actor for Pipe<A, B>
where
    A: Actor,
    B: Actor<Input = A::Output>,
{
    type State = ();
    type Input = A::Input;
    type Output = B::Output;

    fn build(
        self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<Self::Input>,
        async_channel::Receiver<Self::Output>,
    ) {
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

/// Pipe the output of one actor into another.
pub fn pipe<A, B, MA, MB>(first: A, second: B) -> Pipe<A::IntoActor, B::IntoActor>
where
    A: IntoActor<MA>,
    B: IntoActor<MB>,
    A::IntoActor: Actor,
    B::IntoActor: Actor<Input = <A::IntoActor as Actor>::Output>,
{
    Pipe { first: first.into_actor(), second: second.into_actor() }
}

/// Collect inputs into variable-size chunks.
#[cfg(feature = "std")]
#[derive(Clone)]
pub struct Chunk<A> {
    pub(crate) actor: A,
    pub(crate) size: usize,
}

#[cfg(feature = "std")]
impl<A> Actor for Chunk<A>
where
    A: Actor,
{
    type State = ();
    type Input = A::Input;
    type Output = Vec<A::Output>;

    fn build(
        self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<Self::Input>,
        async_channel::Receiver<Self::Output>,
    ) {
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

/// Pipe the output of one actor into another.
#[cfg(feature = "std")]
pub fn chunk<A>(actor: A, size: usize) -> Chunk<A> {
    Chunk { actor, size }
}

/// Run `n` instances of the given actor in parallel.
#[cfg(feature = "std")]
#[derive(Clone)]
pub struct Parallel<A> {
    pub(crate) actor: A,
    pub(crate) workers: usize,
}

#[cfg(feature = "std")]
impl<A> Actor for Parallel<A>
where
    A: Actor + Clone,
{
    type State = ();
    type Input = A::Input;
    type Output = A::Output;

    fn build(
        self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<Self::Input>,
        async_channel::Receiver<Self::Output>,
    ) {
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

/// Filter inputs using a predicate.
#[derive(Clone)]
pub struct Filter<A, P> {
    pub(crate) actor: A,
    pub(crate) predicate: P,
}
impl<A, P> Actor for Filter<A, P>
where
    A: Actor,
    P: Fn(&A::Output) -> bool + Send + Sync + 'static,
{
    type State = ();
    type Input = A::Input;
    type Output = A::Output;

    fn build(
        self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<Self::Input>,
        async_channel::Receiver<Self::Output>,
    ) {
        let (child_fut, child_tx, child_rx) = self.actor.build();
        let (tx, rx) = async_channel::unbounded();
        let predicate = self.predicate;
        let fut = async move {
            loop {
                let item = child_rx.recv().await.unwrap();
                if predicate(&item) {
                    tx.send(item).await.unwrap();
                }
            }
        };
        (child_fut.or(fut), child_tx, rx)
    }
}

/// Pipe the output of one actor through a predicate actor.
pub fn filter<A, P>(actor: A, predicate: P) -> Filter<A, P> {
    Filter { actor, predicate }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "std")]
    use std::time::Duration;

    #[cfg(feature = "std")]
    use rand::Rng;

    use crate::actor::{Actor, IntoActor};

    async fn identity(x: usize) -> usize {
        x
    }

    async fn add1(x: usize) -> usize {
        x + 1
    }

    async fn mul2(x: usize) -> usize {
        x * 2
    }

    #[cfg(feature = "std")]
    async fn sum(items: Vec<usize>) -> usize {
        items.iter().sum()
    }

    #[cfg(feature = "std")]
    async fn delay(x: usize) -> usize {
        // generate the rng then drop it immediately to avoid holding it across thread
        // boundaries
        let millis = { Duration::from_millis(rand::rng().random_range(0..1000)) };
        tokio::time::sleep(millis).await;
        x
    }

    #[tokio::test]
    async fn pipe() {
        add1.into_actor();

        let (task, tx, rx) = add1.pipe(mul2).build();
        tokio::spawn(task);
        tx.send(1).await.unwrap();
        assert_eq!(4, rx.recv().await.unwrap());
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn chunk() {
        let (task, tx, rx) = identity.chunk(3).build();
        tokio::spawn(task);
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        tx.send(3).await.unwrap();
        assert_eq!(vec![1, 2, 3], rx.recv().await.unwrap());

        // with sum
        let (task, tx, rx) = identity.chunk(3).pipe(sum).build();
        tokio::spawn(task);
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        tx.send(3).await.unwrap();
        assert_eq!(6, rx.recv().await.unwrap());
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn parallel() {
        let (task, tx, rx) = delay.parallel(3).build();
        tokio::spawn(task);

        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        tx.send(3).await.unwrap();

        println!("{}", rx.recv().await.unwrap());
        println!("{}", rx.recv().await.unwrap());
        println!("{}", rx.recv().await.unwrap());
    }

    #[tokio::test]
    async fn filter() {
        // Predicate: only allow even numbers
        let is_even = |x: &usize| *x % 2 == 0;
        let (task, tx, rx) = identity.filter(is_even).build();
        tokio::spawn(task);

        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        tx.send(3).await.unwrap();
        tx.send(4).await.unwrap();

        // Only 2 and 4 should pass through
        assert_eq!(2, rx.recv().await.unwrap());
        assert_eq!(4, rx.recv().await.unwrap());
    }
}
