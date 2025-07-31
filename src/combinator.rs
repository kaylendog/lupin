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
        (first_fut.race(second_fut).race(fut), first_in, second_out)
    }
}
