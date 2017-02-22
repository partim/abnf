#[macro_use] extern crate futures;
extern crate tokio_core;

pub use futures::{Async, Poll};
pub use tokio_core::io::EasyBuf;

#[macro_use] pub mod macros;

pub mod ipaddr;
pub mod core;
pub mod parse;
