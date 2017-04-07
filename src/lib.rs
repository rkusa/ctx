extern crate futures;

use std::error::Error;
use std::fmt;
use std::any::Any;
use std::time::Instant;
use futures::{Future, Poll, Async};

mod withcancel;
pub use withcancel::{WithCancel, with_cancel};

#[derive(Debug, PartialEq)]
pub enum ContextError {
    Canceled,
    DeadlineExceeded,
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
        }
    }
}

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
    ///
    /// # Examples
    ///
    /// ```
    /// use context::{Context, with_value, background};
    ///
    /// let a = with_value(background(), 42);
    /// let b = with_value(a, 1.0);
    /// assert_eq!(b.value(), Some(&42));
    /// assert_eq!(b.value(), Some(&1.0));
    /// ```
    fn value<T>(&self) -> Option<&T>
        where T: Any
    {
        None
    }
}

pub struct Background {}

impl Context for Background {}

impl Future for Background {
    type Item = ();
    type Error = ContextError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Async::NotReady)
    }
}

const BACKGROUND: Background = Background {};

pub fn background() -> Background {
    BACKGROUND
}

pub struct WithValue<V, C>
    where C: Context,
          V: Any
{
    parent: Box<C>,
    val: V,
}

impl<V, C> Context for WithValue<V, C>
    where C: Context,
          V: Any
{
    fn deadline(&self) -> Option<Instant> {
        None
    }

    fn value<T>(&self) -> Option<&T>
        where T: Any
    {
        let val_any = &self.val as &Any;
        val_any.downcast_ref::<T>().or_else(|| self.parent.as_ref().value())
    }
}

impl<V, C> Future for WithValue<V, C>
    where C: Context,
          V: Any
{
    type Item = ();
    type Error = ContextError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Async::NotReady)
    }
}

pub fn with_value<V, C>(parent: C, val: V) -> WithValue<V, C>
    where C: Context,
          V: Any
{
    WithValue {
        parent: Box::new(parent),
        val: val,
    }
}

#[test]
fn same_type_test() {
    let a = with_value(background(), 1);
    let b = with_value(a, 2);
    assert_eq!(b.value(), Some(&2));
}

#[test]
fn same_type_workaround_test() {
    #[derive(Debug, PartialEq)]
    struct A(i32);
    #[derive(Debug, PartialEq)]
    struct B(i32);
    let a = with_value(background(), B(1));
    let b = with_value(a, B(2));
    assert_eq!(b.value(), Some(&B(2)));
}