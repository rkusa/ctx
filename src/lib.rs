extern crate futures;

use std::error::Error;
use std::fmt;
use std::any::Any;
use std::time::Instant;
use futures::{Future, Poll, Async};

#[derive(Debug)]
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
    /// let a = with_value(background(), "a", 42);
    /// let b = with_value(a, "b", 1.0);
    /// assert_eq!(b.value("a"), Some(&42));
    /// assert_eq!(b.value("b"), Some(&1.0));
    /// ```
    fn value<I, T>(&self, _: I) -> Option<&T>
        where I: Any + PartialEq,
              T: Any
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

pub struct WithValue<K, V, C>
    where C: Context,
          K: Any + PartialEq,
          V: Any
{
    parent: Box<C>,
    key: K,
    val: V,
}

impl<K, V, C> Context for WithValue<K, V, C>
    where C: Context,
          K: Any + PartialEq,
          V: Any
{
    fn deadline(&self) -> Option<Instant> {
        None
    }

    fn value<I, T>(&self, key: I) -> Option<&T>
        where I: Any + PartialEq,
              T: Any
    {
        let key_equals = {
            let key_any = &key as &Any;
            match key_any.downcast_ref::<K>() {
                Some(k) => {
                    println!("yes! {}", self.key == *k);
                    self.key == *k
                }
                None => false,
            }
        };

        if key_equals {
            let val_any = &self.val as &Any;
            val_any.downcast_ref::<T>()
        } else {
            self.parent.as_ref().value(key)
        }
    }
}

impl<K, V, C> Future for WithValue<K, V, C>
    where C: Context,
          K: Any + PartialEq,
          V: Any
{
    type Item = ();
    type Error = ContextError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Async::NotReady)
    }
}

pub fn with_value<K, V, C>(parent: C, key: K, val: V) -> WithValue<K, V, C>
    where C: Context,
          K: Any + PartialEq,
          V: Any
{
    WithValue {
        parent: Box::new(parent),
        key: key,
        val: val,
    }
}
