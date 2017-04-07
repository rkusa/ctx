///! Ctx defines the Context type, which carries deadlines, cancelation
///! [futures](https://github.com/alexcrichton/futures-rs), and other request-scoped values across API
///! boundaries and between processes.
///!
///! It is similar to Go's [context](https://blog.golang.org/context)
///! [package](https://golang.org/pkg/context/). The main use case is to have incoming requests to a
///! server create a Context. This Context is propagated in the chain of function calls between the
///! incoming request until the outging response. On its way, the Context can be replaced with a
///! derived Context using `with_cancel`, `with_deadline`, `with_timeout`, or `with_value`.

extern crate futures;
extern crate tokio_timer;

use std::error::Error;
use std::fmt;
use std::any::Any;
use std::time::Instant;
use futures::Future;

mod with_value;
mod with_cancel;
mod with_deadline;
pub use with_value::{WithValue, with_value};
pub use with_cancel::{WithCancel, with_cancel};
pub use with_deadline::{WithDeadline, with_deadline, with_timeout};

#[derive(Debug, PartialEq)]
pub enum ContextError {
    Canceled,
    DeadlineExceeded,
    DeadlineTooLong,
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ContextError: {}", self.description())
    }
}

impl Error for ContextError {
    fn description(&self) -> &str {
        match *self {
            ContextError::Canceled => "context has been canceled",
            ContextError::DeadlineExceeded => "deadline has been exceeded",
            ContextError::DeadlineTooLong => "requested deadline too long",
        }
    }
}

/// A Context carries a deadline, a cancelation Future, and other values across API boundaries.
pub trait Context: Future<Item = (), Error = ContextError> {
    /// Returns the time when work done on behalf of this context should be
    /// canceled. Successive calls to deadline return the same result.
    fn deadline(&self) -> Option<Instant> {
        None
    }

    /// Returns the value associated with this context for the expected type.
    ///
    /// Context values should only be used for request-scoped data that transists
    /// processes and API boundaries and not for passing optional parameters to
    /// functions.
    fn value<T>(&self) -> Option<&T>
        where T: Any
    {
        None
    }
}

mod background {
    use {Context, ContextError};
    use futures::{Future, Poll, Async};

    pub struct Background {}

    impl Context for Background {}

    impl Future for Background {
        type Item = ();
        type Error = ContextError;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            Ok(Async::NotReady)
        }
    }

    pub const BACKGROUND: Background = Background {};
}

/// Returns an empty Context. It is never canceled has neither a value nor a deadline. It is
/// typically used as a top-level Context.
pub fn background() -> background::Background {
    background::BACKGROUND
}
