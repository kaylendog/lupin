//! General purpose actors for numerical processing.

use core::ops::AddAssign;

use futures_lite::FutureExt;

use crate::actor::Actor;

/// Trait for actors that process numeric data.
///
/// Provides a method to wrap an actor with cumulative sum functionality.
pub trait NumericActor: Actor {
    /// Wraps the actor with a cumulative sum processor.
    ///
    /// Returns a [`Sum`] actor that accumulates the outputs of `self`.
    fn sum(self) -> Sum<Self>
    where
        Self: Sized,
    {
        Sum { inner: self }
    }
}

impl<A> NumericActor for A where A: Actor {}

/// Actor that accumulates a cumulative sum of outputs from an inner actor.
///
/// Wraps another actor and maintains a running total of its outputs.
/// The sum is updated each time the inner actor produces a new output.
pub struct Sum<A> {
    inner: A,
}

impl<A, U, T> Actor for Sum<A>
where
    A: Actor<Input = U, Output = T>,
    T: AddAssign + Copy + Default,
{
    type State = T;
    type Input = U;
    type Output = T;

    fn build(
        self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<Self::Input>,
        async_channel::Receiver<Self::Output>,
    ) {
        let (in_tx, in_rx) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();
        let (child_fut, child_in, child_out) = self.inner.build();
        let fut = async move {
            let mut state = T::default();
            loop {
                let input = in_rx.recv().await.unwrap();
                child_in.send(input).await.unwrap();
                let item = child_out.recv().await.unwrap();
                state += item;
                out_tx.send(state).await.unwrap();
            }
        };

        (child_fut.or(fut), in_tx, out_rx)
    }
}

/// Functional combinator that creates a cumulative sum actor from any actor.
///
/// This function takes an actor and returns a [`Sum`] actor that accumulates the outputs.
///
/// # Example
/// ```
/// use your_crate::{sum, NumericActor, Actor};
///
/// let my_actor = ...; // some actor implementing Actor
/// let sum_actor = sum(my_actor);
/// ```
pub fn sum<A>(actor: A) -> Sum<A>
where
    A: Actor,
{
    Sum { inner: actor }
}

#[cfg(test)]
mod tests {
    use crate::{
        actor::{Actor, IntoActor},
        numeric::NumericActor,
    };

    async fn one(_: ()) -> usize {
        1
    }

    #[tokio::test]
    async fn test_sum() {
        let (fut, tx, rx) = one.into_actor().sum().build();
        tokio::spawn(fut);
        tx.send(()).await.unwrap();
        tx.send(()).await.unwrap();
        tx.send(()).await.unwrap();

        assert_eq!(rx.recv().await.unwrap(), 1);
        assert_eq!(rx.recv().await.unwrap(), 2);
        assert_eq!(rx.recv().await.unwrap(), 3);
    }
}
