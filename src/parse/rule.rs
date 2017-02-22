//! Rule Parsing.
//!
//! This module heavily relies on closures (and, by extension, functions) as
//! function arguments. There is two types of such closures: *parsing
//! closures* and *converting closures*.
//!
//! Parsing closures attempt to parse a value from the beginning of a buffer.
//! They can succeed, fail, or be undecided. Since the closures are given a
//! mutable reference to a the buffer, it is important that they follow some
//! rules. These are as follows: If the parsing closure succeeds, it must
//! drain the buffer to the end of whatever it successfully parsed. If the
//! parsing closure fails or is undecided, it must not drain anything from
//! the buffer. This is important for parsing closures that combine other
//! parsing closures: If an inner closure succeeds, it will drain the buffer.
//! If then a later inner closure fails leading to the entire outer closure
//! to fail, the outer closure needs to rewind to wherever it started. This
//! can be achieved by wrapping the entire closure inside the `group()`
//! function.
//!
//!
//! # Implementing Rules as ABNF Operators
//!
//! [RFC 5234] defined a number of operators. Here’s how these can be
//! implemented using this module.
//!
//! ## Concatenation: `Rule1 Rule2`
//!
//! Concatenation can be achieved simply by parsing one rule after another
//! returning early if a rule either fails or is undecided using the
//! `try_ready!()` macro. Since you are applying several rules, the new
//! rule needs to be wrapped in `group()`.
//!
//! For instance:
//!
//! ```
//! # #[macro_use] extern crate abnf;
//! # use abnf::{Async, EasyBuf, Poll};
//! # use abnf::parse::rule::group;
//! # struct Res;
//! # struct E;
//! # fn rule1(buf: &mut EasyBuf) -> Poll<Res, E> { Ok(Async::Ready(Res)) }
//! # fn rule2(buf: &mut EasyBuf) -> Poll<Res, E> { Ok(Async::Ready(Res)) }
//! fn concat(buf: &mut EasyBuf) -> Poll<Res, E> {
//!     group(buf, |buf| {
//!         try_ready!(rule1(buf));
//!         try_ready!(rule2(buf));
//!         Ok(Async::Ready(Res))
//!     })
//! }
//! # fn main() { }
//! ```
//!
//!
//! # Alternatives: `Rule1 / Rule2`
//!
//! When parsing alternatives, you can ignore errors until you run out of
//! options. The `try_fail!()` macro helps you with that: It returns early
//! on success or not ready, returning an error. The inner expression should
//! use `()` is its error type to indicate that an error is fine. Also, make
//! sure the inner expressions rewind correctly.
//! 
//! ```
//! # #[macro_use] extern crate abnf;
//! # use abnf::{Async, EasyBuf, Poll};
//! # use abnf::parse::rule::group;
//! # struct Res;
//! # struct E;
//! # fn rule1(buf: &mut EasyBuf) -> Poll<Res, ()> { Ok(Async::Ready(Res)) }
//! # fn rule2(buf: &mut EasyBuf) -> Poll<Res, ()> { Ok(Async::Ready(Res)) }
//! fn alt(buf: &mut EasyBuf) -> Poll<Res, E> {
//!     try_fail!(rule1(buf));
//!     try_fail!(rule2(buf));
//!     Err(E)
//! }
//! # fn main() { }
//! ```
//! 
//!
//! # Optional Repetition: `*Rule`
//!
//! For optional repetition, `Rule` is parsed zero or more times. Generally,
//! when this happens you will want to parse each element and then do
//! something with it. This is what `repeat()` is for. It takes a closure
//! for element parsing and one for element processing. The latter also can
//! also drives repetition by indicating whether more elements should be
//! parsed or a result returned.
//!
//! Here is an example applying a `rule()` as many times as it appears pushing
//! each returned value into a vec.
//!
//! ```
//! # #[macro_use] extern crate abnf;
//! # use abnf::{Async, EasyBuf, Poll};
//! # use abnf::parse::rule::{group, repeat};
//! # struct Res;
//! # struct E;
//! # fn rule(buf: &mut EasyBuf) -> Poll<Res, E> { Ok(Async::Ready(Res)) }
//! fn repeat_rule(buf: &mut EasyBuf) -> Poll<Vec<Res>, E> {
//!     let mut res = Vec::new();
//!     try_ready!(repeat(buf, rule, |item| {
//!         match item {
//!             Ok(item) => {
//!                 res.push(item);
//!                 Ok(Async::NotReady)
//!             }
//!             Err(err) => Ok(Async::Ready(()))
//!         }
//!     }));
//!     Ok(Async::Ready(res))
//! }
//! # fn main() { }
//! ```
//!
//! # Specific and Limited Repititions: `<n>Rule` and `<a>*<b>Rule`
//!
//! Both of these happen relatively rarely on a rule-level, so there are no
//! special functions for them. Instead, you can use `repeat()` and pass a
//! counter into the `combine` closure.
//!
//! For instance, `6rule` could be implemented like so:
//!
//! ```
//! # #[macro_use] extern crate abnf;
//! # use abnf::{Async, EasyBuf, Poll};
//! # use abnf::parse::rule::{group, repeat};
//! # struct Res;
//! # struct E;
//! # fn rule(buf: &mut EasyBuf) -> Poll<Res, E> { Ok(Async::Ready(Res)) }
//! fn six_rule(buf: &mut EasyBuf) -> Poll<Vec<Res>, E> {
//!     let mut res = Vec::new();
//!     let mut count = 0;
//!     try_ready!(repeat(buf, rule, |item| {
//!         count += 1;
//!         match item {
//!             Ok(item) => {
//!                 res.push(item);
//!                 if count == 6 {
//!                     Ok(Async::Ready(()))
//!                 }
//!                 else {
//!                     Ok(Async::NotReady)
//!                 }
//!             }
//!             Err(err) => Err(err)
//!         }
//!     }));
//!     Ok(Async::Ready(res))
//! }
//! # fn main() { }
//! ```
//!
//! # At Least Once Repetition: `1*Rule`
//!
//! For the variant of repetition where there needs to be at least on element,
//! there is a special function: `at_least_once()`. It works very much like
//! `repeat()` but fails if the `parse` closure fails on the first repetition.
//! In order to produce the correct error for this case, it takes yet another
//! closure.
//!
//! ```
//! # #[macro_use] extern crate abnf;
//! # use abnf::{Async, EasyBuf, Poll};
//! # use abnf::parse::rule::{group, at_least_once};
//! # struct Res;
//! # struct E;
//! # fn rule(buf: &mut EasyBuf) -> Poll<Res, E> { Ok(Async::Ready(Res)) }
//! fn rule_at_least_once(buf: &mut EasyBuf) -> Poll<Vec<Res>, E> {
//!     let mut res = Vec::new();
//!     try_ready!(at_least_once(buf, rule,
//!         |item| {
//!             match item {
//!                 Ok(item) => {
//!                     res.push(item);
//!                     Ok(Async::NotReady)
//!                 }
//!                 Err(err) => Ok(Async::Ready(()))
//!             }
//!         },
//!         |_| E
//!     ));
//!     Ok(Async::Ready(res))
//! }
//! # fn main() { }
//! ```
//!
//!
//! ## Optional Sequence: `[RULE]`
//!
//! The `optional()` function serves the purpose of allowing a rule to be
//! applied at most once. It returns an `Option<R>`.
//!
//! So, say we want to parse this: `rule1 [rule2]`. This could look like
//! this:
//!
//! ```
//! # #[macro_use] extern crate abnf;
//! # use abnf::{Async, EasyBuf, Poll};
//! # use abnf::parse::rule::{group, optional};
//! # struct Res1; struct Res2;
//! # struct E;
//! # fn rule1(buf: &mut EasyBuf) -> Poll<Res1, E> { Ok(Async::Ready(Res1)) }
//! # fn rule2(buf: &mut EasyBuf) -> Poll<Res2, E> { Ok(Async::Ready(Res2)) }
//! fn rule1_opt_rule2(buf: &mut EasyBuf) -> Poll<(Res1, Option<Res2>), E> {
//!     group(buf, |buf| {
//!         let res1 = try_ready!(rule1(buf));
//!         let res2 = try_ready!(optional(buf, rule2));
//!         Ok(Async::Ready((res1, res2)))
//!     })
//! }
//! # fn main() { }
//! ```

use ::{Async, EasyBuf, Poll};


//------------ Combining Rules -----------------------------------------------

/// Succeeds if parsing within `op` succeeds or rewinds.
pub fn group<P, T, E>(buf: &mut EasyBuf, parse: P) -> Poll<T, E>
           where P: FnOnce(&mut EasyBuf) -> Poll<T, E> {
    let orig_buf = buf.clone();
    let res = parse(buf);
    match res {
        Ok(Async::NotReady) | Err(_) => *buf = orig_buf,
        _ => {}
    }
    res
}


/// Repetition.
///
/// This combinator is driven by two closures.
///
/// The first one, `parse`, parses an element at a time from the beginning
/// of the buffer given. If it returns non-ready, the whole repetition
/// rewinds and returns non-ready.
///
/// Otherwise, the `parse` closure’s result is transformed into a `Result`
/// and given to the closure `combine` which needs to decide what to do
/// next. If it returns an error, the whole repetition rewinds and results
/// in that error. It it returns a value, the repetition is over producing
/// this result. If it returns non-ready, another iterations is done.
pub fn repeat<P, R, E, C, S, F>(buf: &mut EasyBuf, parse: P, mut combine: C)
                          -> Poll<S, F>
              where P: Fn(&mut EasyBuf) -> Poll<R, E>,
                    C: FnMut(Result<R, E>) -> Poll<S, F> {
    group(buf, |buf| {
        loop {
            let item = try_result!(parse(buf));
            match combine(item) {
                Ok(Async::Ready(res)) => return Ok(Async::Ready(res)),
                Err(err) =>  return Err(err),
                Ok(Async::NotReady) => { }
            }
        }
    })
}


/// Repeat at least once.
///
/// This is like `repeat()`, but if `parse` fails already on the first time,
/// `combine` isn’t called at all but rather `empty`.
pub fn at_least_once<P, R, E, C, S, F, D>(buf: &mut EasyBuf,
                                          parse: P, mut combine: C, error: D)
                                          -> Poll<S, F>
                     where P: Fn(&mut EasyBuf) -> Poll<R, E>,
                           C: FnMut(Result<R, E>) -> Poll<S, F>,
                           D: FnOnce(E) -> F {
    group(buf, |buf| {
        match try_result!(parse(buf)) {
            Err(err) => return Err(error(err)),
            Ok(item) => match combine(Ok(item)) {
                Ok(Async::Ready(res)) => return Ok(Async::Ready(res)),
                Err(err) => return Err(err),
                Ok(Async::NotReady) => { }
            }
        }
        loop {
            let item = try_result!(parse(buf));
            match combine(item) {
                Ok(Async::Ready(res)) => return Ok(Async::Ready(res)),
                Err(err) =>  return Err(err),
                Ok(Async::NotReady) => { }
            }
        }
    })
}


/// An optional rule.
pub fn optional<P, R, E, F>(buf: &mut EasyBuf, parse: P) -> Poll<Option<R>, F>
                where P: FnOnce(&mut EasyBuf) -> Poll<R, E> {
    match parse(buf) {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(some)) => Ok(Async::Ready(Some(some))),
        Err(_) => Ok(Async::Ready(None))
    }
}


/*
//============ Combinators for Token Parsing =================================


//============ Concrete Token Parsers ========================================

pub fn octet(buf: &mut EasyBuf, octet: u8) -> Poll<()> {
    if buf.len() == 0 {
        Ok(Async::NotReady)
    }
    else if buf.as_slice()[0] == octet {
        buf.drain_to(1);
        Ok(Async::Ready(()))
    }
    else {
        Err(Error)
    }
}


pub fn maybe_octet(buf: &mut EasyBuf, octet: u8) -> Poll<bool> {
    if buf.len() == 0 {
        Ok(Async::NotReady)
    }
    else if buf.as_slice()[0] == octet {
        buf.drain_to(1);
        Ok(Async::Ready(true))
    }
    else {
        Ok(Async::Ready(false))
    }
}


//------------ Various Parsing -----------------------------------------------





/// Parses a literal sequence of octets.
///
/// If the start of `buf` matches `literal` case-insensitively, drains the
/// length of `literal` from the buffer and ready-returns.  If `buf` starts
/// with an incomplete match, returns non-ready. Otherwise returns an error.
pub fn literal(buf: &mut EasyBuf, literal: &[u8]) -> Poll<()> {
    use std::cmp::min;
    use std::ascii::AsciiExt;

    let litlen = {
        let len = buf.len();
        let litlen = literal.len();
        let minlen = min(len, litlen);
        let reduced = &buf.as_slice()[..minlen];
        let litreduced = &literal[..minlen];

        if !reduced.eq_ignore_ascii_case(litreduced) {
            return Err(Error)
        }
        else if minlen < litlen {
            return Ok(Async::NotReady)
        }
        litlen
    };
    let _ = buf.drain_to(litlen);
    Ok(Async::Ready(()))
}


pub fn translate_literal<T>(buf: &mut EasyBuf, lit: &[u8], res: T) -> Poll<T> {
    try_ready!(literal(buf, lit));
    Ok(Async::Ready(res))
}


/// Parses an escaped sequence of octets.
///
/// The closure `test` is given a bytes slice and is supposed to return the
/// number of octets at the beginnung of the slice form a single escaped
/// character.
///
/// Upon success, the function returns a buffer of the un-escaped characters.
pub fn escaped<F>(buf: &mut EasyBuf, test: F) -> Poll<EasyBuf>
               where F: Fn(&[u8]) -> Poll<usize> {
    let mut i = 0;
    while i < buf.len() {
        match test(&buf.as_slice()[i..]) {
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Ok(Async::Ready(count)) => {
                assert!(count > 0);
                i += count;
            }
            Err(err) => {
                if i > 0 {
                    return Ok(Async::Ready(buf.drain_to(i)));
                }
                else {
                    return Err(err)
                }
            }
        }
    }
    Ok(Async::NotReady)
}

//------------ Category Octet Parsing ----------------------------------------

/// Parses a single octet that matches a function.
///
/// If `test` returns `true` for first octet in `buf`, drains that octet from
/// `buf` and ready-returns it. If `buf` is empty returns non-ready. If there
/// is a first octet but `test` returns `false`, returns an error.
pub fn cat<F>(buf: &mut EasyBuf, test: P) -> Poll<u8>
           where P: FnOnce(u8) -> bool {
    if buf.len() < 1 {
        Ok(Async::NotReady)
    }
    else {
        let ch = buf.as_slice()[0];
        if test(ch) {
            let _ = buf.drain_to(1);
            Ok(Async::Ready(ch))
        }
        else {
            Err(Error)
        }
    }
}


/// Parses a non-empty sequence of octets matched by a function.
///
/// If `buf` starts with a sequence of at least one octet for which `test`
/// returns `true` which is followed by an octet for which `test` returns
/// `false`, drains the sequence from `buf` and ready-returns it. If there
/// is a sequence but no following octet, returns not ready. If the first
/// octet in `buf` is not matched by `test`, returns an error.
///
/// This function is used for parsing sequences in the middle of a message.
/// If the message can ends with the sequence, use `final_cats()` instead.
pub fn cats<F>(buf: &mut EasyBuf, test: F) -> Poll<EasyBuf>
            where F: Fn(u8) -> bool {
    if buf.len() < 1 {
        return Ok(Async::NotReady)
    }
    let mut end = None;
    for (index, item) in buf.as_slice().iter().enumerate() {
        if !test(*item) {
            end = Some(index);
            break;
        }
    }
    match end {
        None => Ok(Async::NotReady),
        Some(0) => Err(Error),
        Some(end) => Ok(Async::Ready(buf.drain_to(end)))
    }
}


pub fn cats_len<F>(slice: &[u8], test: F) -> Poll<usize>
                where F: Fn(u8) -> bool {
    let mut i = 0;
    while i < slice.len() {
        if !test(slice[i]) {
            if i == 0 {
                return Err(Error)
            }
            else {
                return Ok(Async::Ready(i))
            }
        }
        i += 1;
    }
    Ok(Async::NotReady)
}


/// Parses a final non-empty sequence of octets matched by a function.
///
/// If `buf` consists entirly of a sequence of octets matched by `test`,
/// returns a clone of `buf`. Otherwise returns an error.
pub fn final_cats<F>(buf: &mut EasyBuf, test: F) -> Result<EasyBuf>
                  where F: Fn(u8) -> bool {
    let len = buf.len();
    if len == 0 {
        return Err(Error)
    }
    for item in buf.as_slice().iter() {
        if !test(*item) {
            return Err(Error);
        }
    }
    Ok(buf.drain_to(len))
}


/// Parses an optional sequence of octets matched by a function.
pub fn opt_cats<F>(buf: &mut EasyBuf, test: F) -> Poll<Option<EasyBuf>>
                where F: Fn(u8) -> bool {
    let mut end = None;
    for (index, item) in buf.as_slice().iter().enumerate() {
        if !test(*item) {
            end = Some(index);
            break;
        }
    }
    match end {
        None => Ok(Async::NotReady),
        Some(0) => Ok(Async::Ready(None)),
        Some(end) => Ok(Async::Ready(Some(buf.drain_to(end))))
    }
}


/// Parses an option of at least `n` and at most `m` octets matched by `test`.
///
/// # Panics
///
/// The function panics if `m` is not greater than `n`.
pub fn nm_cats<F>(buf: &mut EasyBuf, n: usize, m: usize, test: F)
                  -> Poll<EasyBuf>
               where F: Fn(u8) -> bool {
    assert!(m > n);
    let mut end = None;
    for (index, item) in buf.as_slice().iter().enumerate() {
        if !test(*item) {
            end = Some(index);
            break;
        }
    }
    match end {
        None => Ok(Async::NotReady),
        Some(end) if end < n || end >= m => Err(Error),
        Some(end) => Ok(Async::Ready(buf.drain_to(end)))
    }
}


pub fn cats_cat<F>(buf: &mut EasyBuf, word: F, delim: F) -> Poll<EasyBuf>
                where F: Fn(u8) -> bool {
    if buf.len() < 2 {
        return Ok(Async::NotReady)
    }
    let mut end = None;
    for (index, item) in buf.as_slice().iter().enumerate() {
        if delim(*item) {
            end = Some(index);
            break;
        }
        else if !word(*item) {
            return Err(Error)
        }
    }
    match end {
        None => Ok(Async::NotReady),
        Some(end) => {
            let res = buf.drain_to(end);
            buf.drain_to(1);
            Ok(Async::Ready(res))
        }
    }
}


//============ Test =========================================================

#[cfg(test)]
mod test {
    use futures::Async;
    use tokio_core::io::EasyBuf;
    use super::*;


    fn buf(slice: &[u8]) -> EasyBuf { Vec::from(slice).into() }

    fn escape_seq(slice: &[u8]) -> Poll<usize> {
        assert!(!slice.is_empty());
        if slice[0] == 33 || (slice[0] >= 35 && slice[0] <= 91)
                          || (slice[0] >= 93 && slice[0] <= 126) {
            return Ok(Async::Ready(1))
        }
        if slice[0] == 92 {
            if slice.len() < 2 {
                return Ok(Async::NotReady)
            }
            if slice[0] >= 32 && slice[0] <= 126 {
                return Ok(Async::Ready(2))
            }
        }
        Err(Error)
    }

    #[test]
    fn test_escape() {
        assert_eq!(escaped(&mut buf(b"foo "), escape_seq),
                   Ok(Async::Ready(buf(b"foo"))));
        assert_eq!(escaped(&mut buf(b"f\\oo\\  "), escape_seq),
                   Ok(Async::Ready(buf(b"f\\oo\\ "))));
    }

    #[test]
    fn test_literal() {
        let mut buf = buf(b"FOO ");
        assert_eq!(literal(&mut buf, b"FOO"), Ok(Async::Ready(())));
        assert_eq!(buf.as_slice(), b" ");
    }
}

*/
