
#[macro_export]
macro_rules! assert_eq_ready {
    ($left:expr, $right:expr) => {
        assert_eq!($left, Ok(Async::Ready($right)))
    }
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

#[macro_export]
macro_rules! alt {
    ( $e:expr, $( $tail:tt )* ) => {{
        match $e {
            Ok($crate::Async::Ready(t)) => Ok($crate::Async::Ready(t.into())),
            Ok($crate::Async::NotReady) => Ok($crate::Async::NotReady),
            Err(_) => {
                alt!( $( $tail )* )
            }
        }
    }};

    ( $e:expr ) => {{
        match $e {
            Ok($crate::Async::Ready(t)) => Ok($crate::Async::Ready(t.into())),
            Ok($crate::Async::NotReady) => Ok($crate::Async::NotReady),
            Err(err) => Err(err),
        }
    }};

}

