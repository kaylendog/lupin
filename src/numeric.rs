//! General purpose actors for numerical processing.

// use futures_lite::future::FutureExt;

// use crate::actor::Actor;

// pub struct Accumulate<A> {
//     actor: A,
//     iterations: usize,
// }

// macro_rules! accumulate_int_impl {
//     ($int:ty) => {
//         impl<A> Actor<$int, $int> for Accumulate<A>
//         where
//             A: Actor<$int, $int>,
//         {
//             type State = ();

//             fn build(
//                 self,
//             ) -> (
//                 impl Future<Output = ()>,
//                 async_channel::Sender<$int>,
//                 async_channel::Receiver<$int>,
//             ) {
//                 let (child_fut, child_tx, child_rx) = self.actor.build();
//                 let (parent_tx, parent_rx) = async_channel::unbounded();
//                 let parent_fut = async move {
//                     loop {
//                         let mut acc = 0;
//                         for _ in 0..self.iterations {
//                             acc += child_rx.recv().await.unwrap();
//                         }
//                         parent_tx.send(acc).await.unwrap();
//                     }
//                 };

//                 (parent_fut.or(child_fut), child_tx, parent_rx)
//             }
//         }
//     };
// }

// accumulate_int_impl!(i8);
// accumulate_int_impl!(i16);
// accumulate_int_impl!(i32);
// accumulate_int_impl!(i64);
// accumulate_int_impl!(u8);
// accumulate_int_impl!(u16);
// accumulate_int_impl!(u32);
// accumulate_int_impl!(u64);

// /// Accumulate actor outputs over a given number of iterations.
// pub fn accumulate<A, T>(actor: A, iterations: usize) -> Accumulate<A>
// where
//     A: Actor<T, T>,
//     T: Send,
// {
//     Accumulate { actor, iterations }
// }
