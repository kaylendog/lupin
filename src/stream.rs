//! Actor types for [`futures_lite::Stream`].

use futures_lite::{Stream, StreamExt};

use crate::actor::{Actor, IntoActor};

/// A wrapper type that turns a [`Stream`] into an actor.
pub struct StreamActor<S> {
    inner: S,
}

/// Marker type for stream-based actors.
struct IsStream;

impl<S> Actor for StreamActor<S>
where
    S: Stream + Unpin,
{
    type State = ();
    type Input = ();
    type Output = S::Item;

    fn build(
        mut self,
    ) -> (
        impl Future<Output = ()>,
        async_channel::Sender<Self::Input>,
        async_channel::Receiver<Self::Output>,
    ) {
        let (in_tx, _) = async_channel::unbounded();
        let (out_tx, out_rx) = async_channel::unbounded();

        let fut = async move {
            loop {
                let Some(next) = self.inner.next().await else {
                    break;
                };
                out_tx.send(next).await.unwrap();
            }
        };
        (fut, in_tx, out_rx)
    }
}

impl<S> IntoActor<IsStream> for S
where
    S: Stream + Unpin,
{
    type IntoActor = StreamActor<S>;

    fn into_actor(self) -> Self::IntoActor {
        StreamActor { inner: self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_lite::stream;

    #[tokio::test]
    async fn iter() {
        let test_stream = stream::iter(vec![1, 2, 3]);
        let stream_actor = test_stream.into_actor();

        let (fut, _in_tx, out_rx) = stream_actor.build();

        tokio::spawn(fut);

        assert_eq!(out_rx.recv().await.unwrap(), 1);
        assert_eq!(out_rx.recv().await.unwrap(), 2);
        assert_eq!(out_rx.recv().await.unwrap(), 3);
        assert!(out_rx.try_recv().is_err());
    }
}
