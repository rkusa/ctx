use std::sync::{Arc, Mutex};
use std::time;
use std::any::Any;
use {Context, InnerContext, ContextError};
use futures::{Future, Poll, Async};
use futures::task::{self, Task};

pub struct WithCancel {
    parent: Context,
    canceled: Arc<Mutex<bool>>, // TODO: Arc necessary?
    handle: Arc<Mutex<Option<Task>>>,
}

impl InnerContext for WithCancel {
    fn deadline(&self) -> Option<time::Instant> {
        None
    }

    fn value(&self) -> Option<&Any> {
        None
    }

    fn parent(&self) -> Option<&Context> {
        self.parent.0.parent()
    }
}

impl Future for WithCancel {
    type Item = ();
    type Error = ContextError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if *self.canceled.lock().unwrap() {
            Err(ContextError::Canceled)
        } else {
            self.parent.0
                .poll()
                .map(|r| {
                    if r == Async::NotReady {
                        // perform any necessary operations in order to get notified in case the
                        // context gets canceled
                        let mut handle = self.handle.lock().unwrap();
                        let must_update = match *handle {
                            Some(ref task) if task.will_notify_current() => false,
                            _ => true,
                        };
                        if must_update {
                            *handle = Some(task::current())
                        }
                    }
                    r
                })
        }
    }
}

/// Returns a copy of parent as a new future, which is closed when the returned cancel function is
/// called or when the parent context's future is resolved – whichever happens first.
///
/// # Example
///
/// ```
/// extern crate ctx;
/// extern crate futures;
///
/// use ctx::{Context, ContextError, with_cancel, background};
/// use futures::future::Future;
///
/// fn main() {
///     let (ctx, cancel) = with_cancel(background());
///     cancel();
///
///     assert_eq!(ctx.wait().unwrap_err(), ContextError::Canceled);
/// }
/// ```
pub fn with_cancel(parent: Context) -> (Context, Box<Fn() + Send>) {
    let canceled = Arc::new(Mutex::new(false));
    let handle = Arc::new(Mutex::new(None));
    let canceled_clone = canceled.clone();
    let handle_clone = handle.clone();

    let ctx = WithCancel {
        parent: parent,
        canceled: canceled,
        handle: handle,
    };
    let cancel = Box::new(move || {
                              let mut canceled = canceled_clone.lock().unwrap();
                              *canceled = true;

                              if let Some(ref task) = *handle_clone.lock().unwrap() {
                                  task.notify();
                              }
                          });
    (Context::new(ctx), cancel)
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use std::thread;
    use tokio_timer::Timer;
    use with_cancel::with_cancel;
    use {background, ContextError};
    use futures::Future;

    #[test]
    fn cancel_test() {
        let (ctx, cancel) = with_cancel(background());
        cancel();

        assert_eq!(ctx.wait().unwrap_err(), ContextError::Canceled);
    }

    #[test]
    fn cancel_parent_test() {
        let (parent, cancel) = with_cancel(background());
        let (ctx, _) = with_cancel(parent);
        cancel();

        assert_eq!(ctx.wait().unwrap_err(), ContextError::Canceled);
    }

    #[test]
    fn example_test() {
        let timer = Timer::default();

        let long_running_process = timer.sleep(Duration::from_secs(2));
        let (ctx, cancel) = with_cancel(background());

        let first = long_running_process
            .map_err(|_| ContextError::DeadlineExceeded)
            .select(ctx);

        thread::spawn(move || {
                          thread::sleep(Duration::from_millis(100));
                          cancel();
                      });

        let result = first.wait();
        assert!(result.is_err());
        match result {
            Err((err, _)) => assert_eq!(err, ContextError::Canceled),
            _ => assert!(false),
        }
    }
}
