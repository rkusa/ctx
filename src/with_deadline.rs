use std::time::{Duration, Instant};
use std::any::Any;
use {Context, InnerContext, ContextError, with_cancel, WithCancel};
use futures::{Future, Poll, Async};
use tokio_timer::{Timer, Sleep};

pub struct WithDeadline<C>
    where C: InnerContext
{
    parent: Context<WithCancel<C>>,
    when: Instant,
    deadline: Sleep,
}

impl<C> InnerContext for WithDeadline<C>
    where C: InnerContext
{
    fn deadline(&self) -> Option<Instant> {
        Some(self.when)
    }

    fn value<T>(&self) -> Option<T>
        where T: Any + Clone
    {
        self.parent.value()
    }
}

impl<C> Future for WithDeadline<C>
    where C: InnerContext
{
    type Item = ();
    type Error = ContextError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.deadline.poll() {
            Ok(Async::Ready(_)) => Err(ContextError::DeadlineExceeded),
            Ok(Async::NotReady) => self.parent.poll(),
            Err(_) => Err(ContextError::DeadlineTooLong),
        }
    }
}

/// Returns `with_timeout(parent, deadline - Instant::now())`.
pub fn with_deadline<C>(parent: Context<C>, deadline: Instant) -> (Context<WithDeadline<C>>, Box<Fn() + Send>)
    where C: InnerContext
{
    with_timeout(parent, deadline - Instant::now())
}

/// Returns a copy of the parent context with the given deadline associated to it. The returned
/// context's future resolves when the deadline expires, the returned cancel function is called,
/// or when the parent context's future resolves – whichever happens first.
///
/// # Example
///
/// ```
/// extern crate ctx;
/// extern crate futures;
///
/// use std::time::Duration;
/// use std::thread;
/// use ctx::{Context, ContextError, with_timeout, background};
/// use futures::future::Future;
///
/// fn main() {
///     let (ctx, _) = with_timeout(background(), Duration::new(0, 50));
///     thread::sleep(Duration::from_millis(100));
///
///     assert_eq!(ctx.wait().unwrap_err(), ContextError::DeadlineExceeded);
/// }
/// ```
pub fn with_timeout<C>(parent: Context<C>, timeout: Duration) -> (Context<WithDeadline<C>>, Box<Fn() + Send>)
    where C: InnerContext
{
    let timer = Timer::default();
    let (parent, cancel) = with_cancel(parent);
    let ctx = WithDeadline{
        parent: parent,
        when: Instant::now() + timeout,
        deadline: timer.sleep(timeout),
    };
    (Context::new(ctx), cancel)
}

#[cfg(test)]
mod test {
    use std::time::{Instant, Duration};
    use std::thread;
    use tokio_timer::Timer;
    use with_deadline::with_timeout;
    use {background, ContextError};
    use futures::Future;

    #[test]
    fn cancel_test() {
        let (ctx, cancel) = with_timeout(background(), Duration::new(2, 0));
        cancel();

        assert_eq!(ctx.wait().unwrap_err(), ContextError::Canceled);
    }

    #[test]
    fn deadline_test() {
        let duration = Duration::new(0, 50);
        let when = Instant::now() + duration;
        let (ctx, _) = with_timeout(background(), duration);

        assert!(ctx.deadline().unwrap() - when < Duration::from_millis(10));

        thread::sleep(Duration::from_millis(100));
        assert_eq!(ctx.wait().unwrap_err(), ContextError::DeadlineExceeded);
    }

    #[test]
    fn deadline_nested_test() {
        let (parent, _) = with_timeout(background(), Duration::from_millis(50));
        let (ctx, _) = with_timeout(parent, Duration::from_secs(10));

        thread::sleep(Duration::from_millis(100));
        assert_eq!(ctx.wait().unwrap_err(), ContextError::DeadlineExceeded);
    }

    #[test]
    fn example_test() {
        let timer = Timer::default();

        let long_running_process = timer.sleep(Duration::from_secs(2));
        let (ctx, _) = with_timeout(background(), Duration::new(0, 100));

        let first = long_running_process
            .map_err(|_| ContextError::Canceled)
            .select(ctx);

        let result = first.wait();
        assert!(result.is_err());
        match result {
            Err((err, _)) => assert_eq!(err, ContextError::DeadlineExceeded),
            _ => assert!(false),
        }
    }

    #[test]
    fn clone_test() {
        let duration = Duration::new(0, 50);
        let when = Instant::now() + duration;
        let (ctx, _) = with_timeout(background(), duration);
        let clone = ctx.clone();

        assert!(clone.deadline().unwrap() - when < Duration::from_millis(10));

        thread::sleep(Duration::from_millis(100));
        assert_eq!(ctx.wait().unwrap_err(), ContextError::DeadlineExceeded);
        assert_eq!(clone.wait().unwrap_err(), ContextError::DeadlineExceeded);
    }
}
