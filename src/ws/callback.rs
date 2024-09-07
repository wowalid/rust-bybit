
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// Define a trait that supports async functions
pub trait Arg {
    type ValueType<'a>: Deserialize<'a>
    where
        Self: 'a;
}

pub trait Callback<A: Arg>: for<'any> FnMut(A::ValueType<'any>) -> Pin<Box<dyn Future<Output = ()> + 'static>> {}
impl<A: Arg, F: for<'any> FnMut(A::ValueType<'any>) -> Pin<Box<dyn Future<Output = ()> + 'static>>> Callback<A> for F {}

