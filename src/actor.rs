use async_trait::async_trait;

#[async_trait]
pub trait Actor<I, O>
where
    I: Send,
    O: Send,
{
    async fn process<'a>(&'a mut self, input: I) -> O
    where
        I: 'a;
}

#[async_trait]
impl<I, O, F, Fut> Actor<I, O> for F
where
    I: Send,
    O: Send,
    F: Fn(I) -> Fut + Send,
    Fut: Future<Output = O> + Send,
{
    async fn process<'a>(&'a mut self, input: I) -> O
    where
        I: 'a,
    {
        (self)(input).await
    }
}
