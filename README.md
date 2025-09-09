# Lupin

A stupendously simple actor framework for the functionally inclined.

## Overview

Lupin is a lightweight actor framework for building composable and modular asynchronous systems. It is built around the concept of **Actors**: asynchronous functions that process input messages and produce output messages. Actors can be composed into pipelines, enabling the creation of complex workflows.

For example, the simplest actor possible, the identity actor, can be defined as an asynchronous function that returns its input:

```rust
async fn identity(input: &usize) -> usize {
    *input
}
```

More complex actors can be built in a similar fashion:

```rust
async fn collatz(input: &usize) -> usize {
    if *input % 2 == 0 {
        *input / 2
    } else {
        3 * *input + 1
    }
}
```

An actor runs indefinitely as long as input is available.

## Types of Actor

Lupin distinguishes between two primary kinds of actors:

- **Source Actors**: These actors do not require external input to produce output. They are defined as asynchronous functions with no input argument. Source actors are useful for generating data streams, timers, or periodic events.

  ```rust
  async fn source() -> usize {
      42
  }
  let (task, actor) = source.build();
  tokio::spawn(task);
  let value = actor.recv().await.unwrap();
  ```

- **Pipeline Actors**: These actors process input messages and produce output messages. They are defined as asynchronous functions that take an input (and optionally mutable state) and return an output.

  ```rust
  async fn add1(input: &usize) -> usize {
      *input + 1
  }
  let (task, actor) = add1.build();
  tokio::spawn(task);
  actor.send(41).await.unwrap();
  let value = actor.recv().await.unwrap(); // 42
  ```

## Interacting with Actors

When you build an actor, Lupin returns an `ActorRef`, which is an enum representing a handle to the actor’s communication channels. `ActorRef` allows you to send input messages to pipeline actors and receive output messages from both source and pipeline actors.

- For **Source actors**, you use `.recv().await` to pull output values.
- For **Pipeline actors**, you use `.send(input).await` to provide input, and `.recv().await` to get the output.

`ActorRef` abstracts away the underlying channels and provides a unified API for interacting with actors, making it easy to compose and connect actors in your system.

## Combinators

Lupin provides a rich set of combinators to compose and transform actors:

- **`pipe`**: Connects the output of one actor to the input of another.
- **`chunk`**: Groups outputs into fixed-size chunks. (Requires `alloc` feature.)
- **`parallel`**: Runs multiple instances of an actor in parallel. (Requires `alloc` feature.)
- **`filter`**: Filters outputs based on a predicate.
- **`map`**: Transforms outputs using a mapping function.
- **`filter_map`**: Combines filtering and mapping in a single step.

For example, you can compose two actors together using `pipe` to perform two sequential operations on an input:

```rust
async fn mul3(input: &usize) -> usize {
    *input * 3
}

async fn add1(input: &usize) -> usize {
    *input + 1
}

let (task, actor) = mul3.pipe(add1).build();
tokio::spawn(task);
actor.send(2).await.unwrap();
let result = actor.recv().await.unwrap();
assert_eq!(result, 7);
```

## Embedded Environments

Since `lupin` uses `futures_lite`, it supports `no_std` environments out of the box, optionally without allocation. Features requiring allocation can be disabled in `Cargo.toml`:

```toml
[dependencies.lupin]
default-features = false
```

Some combinators or utilities that require OS-level async runtimes (like Tokio) or allocation may not be available, but core actor composition and message processing remain fully supported.

## Mutable Actors

In addition to immutable actors, Lupin supports **mutable actors** for scenarios where state needs to be maintained across messages. Mutable actors are defined as asynchronous functions that operate on a mutable state and process input messages.

For example, a counter actor that increments its state with each input can be defined as:

```rust
async fn counter(state: &mut usize, input: &usize) -> usize {
    *state += *input;
    *state
}
```

Mutable actors are particularly useful for tasks like aggregation, caching, or any operation that requires stateful computation.

## Examples

### Pipeline Composition

Compose two actors into a pipeline where the output of one actor becomes the input of the next:

```rust
async fn add1(input: &usize) -> usize { *input + 1 }
async fn mul2(input: &usize) -> usize { *input * 2 }

let (task, actor) = add1.pipe(mul2).build();
tokio::spawn(task);
actor.send(1).await.unwrap();
let result = actor.recv().await.unwrap();
assert_eq!(result, 4);
```

### Chunking Outputs

Group outputs into fixed-size chunks (requires `alloc` feature):

```rust
let (task, actor) = identity.chunk(3).build();
tokio::spawn(task);
actor.send(1).await.unwrap();
actor.send(2).await.unwrap();
actor.send(3).await.unwrap();
let chunk = actor.recv().await.unwrap();
assert_eq!(chunk, vec![1, 2, 3]);
```

## Contributing

Contributions to Lupin are welcome and encouraged! Please read [CONTRIBUTING.md](./CONTRIBUTING.md) for more information.

## License

This project is licensed under a dual MIT and Apache 2.0 license, at your discretion. See [LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE-2.0](./LICENSE-APACHE-2.0) for more information.
