use std::any::Any;
use std::time::Instant;
use {Context, ContextError};
use futures::{Future, Poll, Async};

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

/// Returns a copy of parent, but with the given value associated to it.
///
/// Context values should only be used for request-scoped data that transists
/// processes and API boundaries and not for passing optional parameters to
/// functions.
///
/// It is recommended to use structs as values instead of simple data types
/// like strings and ints to be very specific of what result to expect when
/// retrieving a value. Having values of the same data type among the ancestors
/// would always return the first hit.
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
pub fn with_value<V, C>(parent: C, val: V) -> WithValue<V, C>
    where C: Context,
          V: Any
{
    WithValue {
        parent: Box::new(parent),
        val: val,
    }
}


#[cfg(test)]
mod test {
    use with_value::with_value;
    use {Context, background};

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
}