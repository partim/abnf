
#[macro_export]
macro_rules! assert_eq_ready {
    ($left:expr, $right:expr) => {
        assert_eq!($left, Ok(Async::Ready($right)))
    }
}


/// A macro for extracting the successful type of a `Poll<T, E>`.
///
/// This macro bakes propagation of both errors and `NotReady` signals by
/// returning early.
/// 
/// This is identical to the macro by the same name defined by the
/// `futures` crate.
#[macro_export]
macro_rules! try_ready {
    ($e:expr) => (match $e {
        Ok($crate::Async::Ready(t)) => t,
        Ok($crate::Async::NotReady) => return Ok($crate::Async::NotReady),
        Err(e) => return Err(From::from(e)),
    })
}


/// A macro for extracting the result from a `Poll<T, E>`.
///
/// Turns the `Poll<T, E>` into a `Result<T, E>` by early returning a
/// possible `Ok(Async::NotReady)`.
#[macro_export]
macro_rules! try_result {
    ($e:expr) => (match $e {
        Ok($crate::Async::Ready(t)) => Ok(t),
        Ok($crate::Async::NotReady) => return Ok($crate::Async::NotReady),
        Err(e) => Err(e),
    })
}

/// A macro for extracting the result from a `Poll<T, E>`.
///
/// Turns the `Poll<T, E>` into a `Result<T, E>` by early returning a
/// possible `Ok(Async::NotReady)`.
#[macro_export]
macro_rules! try_fail {
    ($e:expr) => (match $e {
        Ok($crate::Async::Ready(t)) => return Ok($crate::Async::Ready(t)),
        Ok($crate::Async::NotReady) => return Ok($crate::Async::NotReady),
        Err(e) => e,
    })
}

