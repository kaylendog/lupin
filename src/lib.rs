//! A runtime-agnostic composable actor framework.
//!
//! The actor model provides an abstraction over asynchronous systems using actors - modular processes that can:
//!
//!  - Send and receive messages to other actors;
//! - Create new actors and destroy old ones;
//! - Update their internal state.

mod actor;
mod channel;
mod combinator;
mod service;

pub use actor::Actor;
pub use combinator::repeat;
pub use service::{Service, ServiceExt, once, source};

#[cfg(test)]
mod test {
    use crate::service::source;

    #[test]
    fn test_pipe() {
        let service = source(|_: ()| async { () });
    }
}
