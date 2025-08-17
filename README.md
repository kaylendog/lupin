# Lupin

A stupendously simple actor framework for the functionally inclined.

## Overview

Lupin is a light-weight actor framework for building composable and modular asynchronous systems. It is built around the concept of **Actors**: asynchronous function that processes input messages and produces output messages. Actors can be composed into pipelines, enabling the creation of complex workflows.

For example, the simplest actor possible, the identity actor, can be defined as an asynchronous function that returns its input:

```rust
async fn identity(input: usize) -> usize {
    input
}
```

More complex actors can be built in a similar fashion:

```rs
async fn collatz(input: usize) -> usize {
    if input % 2 == 0 {
        input / 2
    } else {
        3 * input + 1
    }
}
```

An actor runs indefinitely as long as input is available.

## Combinators

Lupin provides a rich set of combinators to compose and transform actors:

- **`pipe`**: Connects the output of one actor to the input of another.
- **`chunk`**: Groups outputs into fixed-size chunks.
- **`parallel`**: Runs multiple instances of an actor in parallel.
- **`filter`**: Filters outputs based on a predicate.
- **`map`**: Transforms outputs using a mapping function.
- **`filter_map`**: Combines filtering and mapping in a single step.

## Mutable Actors

In addition to immutable actors, Lupin supports **mutable actors** for scenarios where state needs to be maintained across messages. Mutable actors are defined as asynchronous functions that operate on a mutable state and process input messages.

For example, a counter actor that increments its state with each input can be defined as:

```rust
async fn counter(state: State<'_, usize>, input: usize) -> usize {
    *state += input;
    *state
}
```

Mutable actors are particularly useful for tasks like aggregation, caching, or any operation that requires stateful computation.

## Examples

### Pipeline Composition

Compose two actors into a pipeline where the output of one actor becomes the input of the next:

```rust
let (task, tx, rx) = add1.pipe(mul2).build();
tokio::spawn(task);
tx.send(1).await.unwrap();
let result = rx.recv().await.unwrap();
assert_eq!(result, 4);
```

### Chunking Outputs

Group outputs into fixed-size chunks:

```rust
let (task, tx, rx) = identity.chunk(3).build();
tokio::spawn(task);
tx.send(1).await.unwrap();
tx.send(2).await.unwrap();
tx.send(3).await.unwrap();
let chunk = rx.recv().await.unwrap();
assert_eq!(chunk, vec![1, 2, 3]);
```

## Contributing

Contributions to Lupin are welcome and encouraged! Please read [CONTRIBUTING.md](./CONTRIBUTING.md) for more information.

## License

This project is licensed under a dual MIT and Apache 2.0 license, at your discretion. See [LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE-2.0](./LICENSE-APACHE-2.0) for more information.
