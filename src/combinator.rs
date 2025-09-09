//! Combinatorial actors that enable larger systems to be built by connecting
//! actors together.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use core::marker::PhantomData;

use futures_lite::FutureExt;

use crate::actor::{Actor, ActorRef};

/// Pipes the output of one actor into another.
///
/// This combinator allows you to connect two actors such that the output of the
/// first is sent as input to the second. The `Pipe` struct holds both actors
/// internally.
#[derive(Clone)]
pub struct Pipe<A, B> {
    pub(crate) first: A,
    pub(crate) second: B,
}

impl<MA, MB, A, B> Actor<(MA, MB)> for Pipe<A, B>
where
    A: Actor<MA>,
    B: Actor<MB, Input = A::Output>,
{
    type State = ();
    type Input = A::Input;
    type Output = B::Output;

    fn build(self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>) {
        let (first_fut, first_ref) = self.first.build();
        let (second_fut, second_ref) = self.second.build();
        let (out_ref, in_rx, out_tx) = first_ref.pipe(second_ref);
        let fut = async move {
            loop {
                out_tx.send(in_rx.recv().await.unwrap()).await.unwrap();
            }
        };
        (first_fut.or(fut).or(second_fut), out_ref)
    }
}

/// Pipe the output of one actor into another.
pub fn pipe<A, B, MA, MB>(first: A, second: B) -> Pipe<A, B>
where
    A: Actor<MA>,
    B: Actor<MB, Output = A::Input>,
{
    Pipe { first, second }
}

/// Collect inputs into variable-size chunks.
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct Chunk<A> {
    pub(crate) actor: A,
    pub(crate) size: usize,
}

#[cfg(feature = "alloc")]
impl<MA, A> Actor<MA> for Chunk<A>
where
    A: Actor<MA>,
{
    type State = ();
    type Input = A::Input;
    type Output = Vec<A::Output>;

    fn build(self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>) {
        let (child_fut, child_ref) = self.actor.build();
        let (out_ref, rx, tx) = child_ref.splice();

        let fut = async move {
            let mut buffer = Vec::with_capacity(self.size);
            loop {
                // fill buffer to capacity
                buffer.push(rx.recv().await.unwrap());
                if buffer.len() < self.size {
                    continue;
                }
                // send chunk
                tx.send(buffer).await.unwrap();
                buffer = Vec::with_capacity(self.size);
            }
        };
        (child_fut.or(fut), out_ref)
    }
}

/// Pipe the output of one actor into another.
#[cfg(feature = "alloc")]
pub fn chunk<A>(actor: A, size: usize) -> Chunk<A> {
    Chunk { actor, size }
}

/// Run `n` instances of the given actor in parallel.
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct Parallel<A> {
    pub(crate) actor: A,
    pub(crate) workers: usize,
}

#[cfg(feature = "alloc")]
/// Creates a `Parallel` actor that runs `n` instances of the given actor in
/// parallel.
pub fn parallel<MA, A>(actor: A, workers: usize) -> Parallel<A>
where
    A: Actor<MA> + Clone,
{
    Parallel { actor, workers }
}

impl<MA, A, F, B> Actor<MA> for Map<A, F>
where
    A: Actor<MA>,
    F: Fn(A::Output) -> B + Send + Sync + 'static,
    B: Send + 'static,
{
    type State = ();
    type Input = A::Input;
    type Output = B;

    fn build(self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>) {
        let (child_fut, child_ref) = self.actor.build();
        let (out_ref, rx, tx) = child_ref.splice();
        let mapper = self.func;
        let fut = async move {
            loop {
                let item = rx.recv().await.unwrap();
                tx.send(mapper(item)).await.unwrap();
            }
        };
        (child_fut.or(fut), out_ref)
    }
}

/// Apply a mapping function to the output of an actor.
pub fn map<MA, A, F, B>(actor: A, mapper: F) -> Map<A, F>
where
    A: Actor<MA>,
    F: Fn(A::Output) -> B + Send + Sync + 'static,
    B: Send + 'static,
{
    Map { actor, func: mapper }
}

/// Filter inputs using a predicate.
#[derive(Clone)]
pub struct Filter<A, P> {
    pub(crate) actor: A,
    pub(crate) predicate: P,
}

impl<MA, A, P> Actor<MA> for Filter<A, P>
where
    A: Actor<MA>,
    P: Fn(&A::Output) -> bool + Send + Sync + 'static,
{
    type State = ();
    type Input = A::Input;
    type Output = A::Output;

    fn build(self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>) {
        let (child_fut, child_ref) = self.actor.build();
        let (out_ref, rx, tx) = child_ref.splice();
        let predicate = self.predicate;
        let fut = async move {
            loop {
                let item = rx.recv().await.unwrap();
                if predicate(&item) {
                    tx.send(item).await.unwrap();
                }
            }
        };
        (child_fut.or(fut), out_ref)
    }
}

/// Pipe the output of one actor through a predicate actor.
pub fn filter<A, P>(actor: A, predicate: P) -> Filter<A, P> {
    Filter { actor, predicate }
}

/// Transform the output of an actor using a mapping function.
#[derive(Clone)]
pub struct Map<A, F> {
    pub(crate) actor: A,
    pub(crate) func: F,
}

/// Transform the output of an actor using a filter and mapping function.
#[derive(Clone)]
pub struct FilterMap<A, F> {
    pub(crate) actor: A,
    pub(crate) func: F,
}

impl<MA, A, F, B> Actor<MA> for FilterMap<A, F>
where
    A: Actor<MA>,
    F: Fn(A::Output) -> Option<B> + Send + Sync + 'static,
    B: Send + 'static,
{
    type State = ();
    type Input = A::Input;
    type Output = B;

    fn build(self) -> (impl Future<Output = ()>, ActorRef<Self::Input, Self::Output>) {
        let (child_fut, child_ref) = self.actor.build();
        let (out_ref, rx, tx) = child_ref.splice();
        let filter_mapper = self.func;
        let fut = async move {
            loop {
                let item = rx.recv().await.unwrap();
                if let Some(mapped) = filter_mapper(item) {
                    tx.send(mapped).await.unwrap();
                }
            }
        };
        (child_fut.or(fut), out_ref)
    }
}

/// Apply a filter and mapping function to the output of an actor.
pub fn filter_map<MA, A, F, B>(actor: A, filter_mapper: F) -> FilterMap<A, F>
where
    A: Actor<MA>,
    F: Fn(A::Output) -> Option<B> + Send + Sync + 'static,
    B: Send + 'static,
{
    FilterMap { actor, func: filter_mapper }
}

/// Broadcasts each input to multiple output receivers.
///
/// This actor receives inputs and sends each input to all output receivers.
/// Useful for scenarios where multiple consumers need to receive the same data.
#[cfg(feature = "alloc")]
pub struct Broadcast<I, O, A> {
    pub(crate) __marker: PhantomData<(I, O)>,
    pub(crate) actors: A,
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "alloc")]
    use alloc::vec::Vec;
    use core::time::Duration;

    use crate::actor::Actor;

    async fn identity(x: &usize) -> usize {
        *x
    }

    async fn add1(x: &usize) -> usize {
        x + 1
    }

    async fn mul2(x: &usize) -> usize {
        x * 2
    }

    #[cfg(feature = "alloc")]
    async fn sum(items: &Vec<usize>) -> usize {
        items.iter().sum()
    }

    #[cfg(feature = "alloc")]
    async fn delay(x: &usize) -> usize {
        // generate the rng then drop it immediately to avoid holding it across thread
        // boundaries
        let millis = {
            use rand::Rng;
            Duration::from_millis(rand::rng().random_range(0..1000))
        };
        tokio::time::sleep(millis).await;
        *x
    }

    #[tokio::test]
    async fn pipe() {
        let (task, actor) = add1.pipe(mul2).build();
        tokio::spawn(task);
        actor.send(1).await.unwrap();
        assert_eq!(4, actor.recv().await.unwrap());
    }

    #[cfg(feature = "alloc")]
    #[tokio::test]
    async fn chunk() {
        use alloc::vec;

        let (task, actor) = identity.chunk(3).build();
        tokio::spawn(task);
        actor.send(1).await.unwrap();
        actor.send(2).await.unwrap();
        actor.send(3).await.unwrap();
        assert_eq!(vec![1, 2, 3], actor.recv().await.unwrap());

        // with sum
        let (task, actor) = identity.chunk(3).pipe(sum).build();
        tokio::spawn(task);
        actor.send(1).await.unwrap();
        actor.send(2).await.unwrap();
        actor.send(3).await.unwrap();
        assert_eq!(6, actor.recv().await.unwrap());
    }

    // #[cfg(feature = "alloc")]
    // #[tokio::test]
    // async fn parallel() {
    //     let (task, actor) = delay.parallel(3).build();
    //     tokio::spawn(task);

    //     actor.send(1).await.unwrap();
    //     actor.send(2).await.unwrap();
    //     actor.send(3).await.unwrap();
    // }

    #[tokio::test]
    async fn filter() {
        // Predicate: only allow even numbers
        let (task, actor) = identity.filter(|x: &usize| *x % 2 == 0).build();
        tokio::spawn(task);

        actor.send(1).await.unwrap();
        actor.send(2).await.unwrap();
        actor.send(3).await.unwrap();
        actor.send(4).await.unwrap();

        // Only 2 and 4 should pass through
        assert_eq!(2, actor.recv().await.unwrap());
        assert_eq!(4, actor.recv().await.unwrap());
    }

    #[tokio::test]
    async fn map() {
        // Mapping function: multiply by 10
        let (task, actor) = identity.map(|x| x * 10).build();
        tokio::spawn(task);

        actor.send(1).await.unwrap();
        actor.send(2).await.unwrap();
        actor.send(3).await.unwrap();

        assert_eq!(10, actor.recv().await.unwrap());
        assert_eq!(20, actor.recv().await.unwrap());
        assert_eq!(30, actor.recv().await.unwrap());
    }

    #[tokio::test]
    async fn filter_map() {
        // Filter and map function: only allow even numbers and divide them by 2
        let (task, actor) =
            identity.filter_map(|x: usize| if x % 2 == 0 { Some(x / 2) } else { None }).build();
        tokio::spawn(task);

        actor.send(1).await.unwrap();
        actor.send(2).await.unwrap();
        actor.send(3).await.unwrap();
        actor.send(4).await.unwrap();

        // Only 2 and 4 should pass through, mapped to 1 and 2 respectively
        assert_eq!(1, actor.recv().await.unwrap());
        assert_eq!(2, actor.recv().await.unwrap());
    }
}
