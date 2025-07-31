# fenrir

A stupendously simple actor framework for the functionally inclined.

## Overview

The core of Fenrir's abstraction is built on two key concepts:

- Actors
- Services

### Actors

Actors are asynchronous, impure functions that recive a single input `I` and produce an output `O`.

```rs
async fn actor(input: usize) -> usize {
  input + 1
}
```

An actor will run indefinitely as long as input is available.

### Services

A service, like an actor, can receive input `I` and produces output `O`.

## License

This project is licensed under a dual MIT and Apache 2.0 license, at your discretion. See [LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE-2.0](./LICENSE-APACHE-2.0) for more information.
