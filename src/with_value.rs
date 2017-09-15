use std::any::Any;
use {Context, InnerContext, ContextError};
use futures::{Future, Poll};

pub struct WithValue<V>
where
    V: Any,
{
    parent: Context,
    val: V,
}

impl<V> InnerContext for WithValue<V>
where
    V: Any,
{
    fn value(&self) -> Option<&Any> {
        let val_any = &self.val as &Any;
        Some(val_any)
    }

    fn parent(&self) -> Option<&Context> {
        Some(&self.parent)
    }
}

impl<V> Future for WithValue<V>
where
    V: Any,
{
    type Item = ();
    type Error = ContextError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.parent.0.poll()
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
/// use ctx::{Context, with_value, background};
///
/// let a = with_value(background(), 42);
/// let b = with_value(a, 1.0);
/// assert_eq!(b.value(), Some(42));
/// assert_eq!(b.value(), Some(1.0));
/// ```
pub fn with_value<V>(parent: Context, val: V) -> Context
where
    V: Any,
{
    Context::new(WithValue {
        parent: parent,
        val: val,
    })
}

#[cfg(test)]
mod test {
    use with_value::with_value;
    use with_cancel::with_cancel;
    use {background, ContextError};
    use futures::Future;

    #[test]
    fn poll_parent_test() {
        let (parent, cancel) = with_cancel(background());
        let ctx = with_value(parent, 42);
        cancel();

        assert_eq!(ctx.wait().unwrap_err(), ContextError::Canceled);
    }

    #[test]
    fn same_type_2test() {
        let a = with_value(background(), 42);
        let b = with_value(a, 1.0);
        assert_eq!(b.value(), Some(42));
        assert_eq!(b.value(), Some(1.0));
    }

    #[test]
    fn same_type_test() {
        let a = with_value(background(), 1);
        let b = with_value(a, 2);
        assert_eq!(b.value(), Some(2));
    }

    #[test]
    fn same_type_workaround_test() {
        #[derive(Debug, PartialEq, Clone)]
        struct A(i32);
        #[derive(Debug, PartialEq, Clone)]
        struct B(i32);
        let a = with_value(background(), A(1));
        let b = with_value(a, B(1));
        assert_eq!(b.value(), Some(A(1)));
    }
}
