use fenrir::{
    buffer::compression::{Zstd, compress, decompress},
    prelude::*,
};

#[tokio::test]
async fn compression() {
    let (task, tx, rx) = compress::<Zstd>().pipe(decompress::<Zstd>()).build();
    tokio::spawn(task);

    for _ in 0..10_000 {
        tx.send(vec![0u8; 32768]).await.unwrap();
    }

    for _ in 0..10_000 {
        rx.recv().await.unwrap();
    }
}
