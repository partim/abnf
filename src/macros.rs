
/// A macro for extracting the result from a `Poll<T, E>`.
///
/// Turns the `Poll<T, E>` into a `Result<T, F>` by early returning a
/// possible `Ok(Async::NotReady)` and calling `F::from()` for an error.
#[macro_export]
macro_rules! try_result {
    ($e:expr) => (match $e {
        Ok($crate::Async::Ready(t)) => Ok(t),
        Ok($crate::Async::NotReady) => return Ok($crate::Async::NotReady),
        Err(e) => Err(From::from(e)),
    })
}

