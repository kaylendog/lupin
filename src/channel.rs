use async_trait::async_trait;

/// An asynchronous channel that can be read from.
#[async_trait]
pub trait ChannelRead<O> {
    /// Read from the channel.
    async fn read(&mut self) -> Option<O>;
}

/// An asynchronous channel that can be written to.
#[async_trait]
pub trait ChannelWrite<I> {
    /// Write to the channel.
    async fn write<'input>(&'input mut self, input: I)
    where
        I: 'input;
}

/// Represents a bidiretional chanenl.
pub trait BidirectionalChannel<I, O>: ChannelWrite<I> + ChannelRead<O> {
    /// Convert this channel into its write half.
    fn into_write<R: ChannelWrite<I>>(self) -> R
    where
        Self: Sized;

    /// Convert this channel into its read half.
    fn into_read<R: ChannelRead<O>>(self) -> R
    where
        Self: Sized;
}

/// A fan-out channel.
pub struct Fan<A, B>
where
    A: Clone,
{
    a_tx: async_broadcast::Sender<A>,
    a_rx: async_broadcast::Receiver<A>,
    b_tx: async_channel::Sender<B>,
    b_rx: async_channel::Receiver<B>,
}

/// The fan-out side of the.
pub struct FanOut<I, O> {
    tx: async_broadcast::Sender<I>,
    rx: async_channel::Receiver<O>,
}

#[derive(Clone)]
pub struct FanIn<I, O> {
    tx: async_channel::Sender<I>,
    rx: async_broadcast::Receiver<O>,
}

impl<A, B> Fan<A, B>
where
    A: Clone,
{
    pub fn split(self) -> (FanOut<A, B>, FanIn<B, A>) {
        (
            FanOut {
                tx: self.a_tx,
                rx: self.b_rx,
            },
            FanIn {
                tx: self.b_tx,
                rx: self.a_rx,
            },
        )
    }
}
