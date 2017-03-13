//! Token Parsing
//!
//! This module defines the type `Token` on which all token parsing is based.
//! It also provides a number of stand-alone functions. For token parsing.
//!
//! These functions fall in two categories: there are functions that operate
//! atop a token and functions that operate atop a buffer. The former are
//! meant to be combined into composite functions that parse tokens. The
//! latter are meant to take such composite functions and apply them to the
//! beginning of a buffer. The latter are easily recognizable by having one
//! of two prefixes: `parse_` for functions that return the content of the
//! token either as a buffer or some other appropriate type and `skip_` for
//! functions that silently skip over the token.

use std::ops;
use ::{Async, EasyBuf, Poll};


//============ Basic Token Parsing ===========================================

//------------ Token --------------------------------------------------------

/// A token in the process of being parsed.
///
/// A token is parsed from the beginning of an `EasyBuf` by advancing over
/// octets until the token’s end is discoverd at which point the token can
/// be drained from the buffer and converted into an `EasyBuf` of its own.
pub struct Token<'a> {
    buf: &'a mut EasyBuf,
    start: usize
}


impl<'a> Token<'a> {
    /// Creates a new token atop the given buffer.
    pub fn new(buf: &'a mut EasyBuf) -> Self {
        Token {
            buf: buf,
            start: 0
        }
    }

    /// Returns a bytes slice of what hasn’t been advanced over yet.
    pub fn as_slice(&self) -> &[u8] {
        &self.buf.as_slice()[self.start..]
    }

    /// Advances the token by `count` octets.
    ///
    /// # Panic
    ///
    /// The method panics if `count` would advance beyond the end of the
    /// underlying buffer.
    pub fn advance(&mut self, count: usize) {
        assert!(self.start + count <= self.buf.len());
        self.start += count;
    }

    /// Advances one octet if `test` returned `true` for it.
    ///
    /// Ready-returns if there was at least one octet available with the
    /// result of the test closure. Returns non-ready if there are no more
    /// octets in the buffer. Never returns an error.
    pub fn advance_if<F, E>(&mut self, test: F) -> Poll<bool, E>
                      where F: FnOnce(u8) -> bool {
        let res = test(try_ready!(self.first()));
        if res {
            self.advance(1)
        }
        Ok(Async::Ready(res))
    }

    /// Advances one octet if `test` succeeds, producing an error otherwise.
    ///
    /// This behaves like `advance()` except that if `test` returns false,
    /// the closure `error` is called and its result returned.
    pub fn expect<P, Q, E>(&mut self, test: P, error: Q) -> Poll<(), E>
                  where P: FnOnce(u8) -> bool,
                        Q: FnOnce() -> E {
        let res = test(try_ready!(self.first()));
        if res {
            self.advance(1);
            Ok(Async::Ready(()))
        }
        else {
            Err(error())
        }
    }

    /// Returns the first remaining character of the buffer if available.
    pub fn first<E>(&self) -> Poll<u8, E> {
        if self.start < self.buf.len() {
            Ok(Async::Ready(self.buf.as_slice()[self.start]))
        }
        else {
            Ok(Async::NotReady)
        }
    }

    /// Drains the token from the underlying buffer.
    pub fn drain(self) -> EasyBuf {
        self.buf.drain_to(self.start)
    }

    /// Drops the token from the underlying buffer.
    pub fn skip(self) {
        let _  = self.buf.drain_to(self.start);
    }
}


//--- Deref

impl<'a> ops::Deref for Token<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}


//------------ Essential Token Parsing Functions -----------------------------

/// Parses a token from the beginning of a buffer.
///
/// The closure `parseop` is given a token atop `buf`. If the closure returns
/// ready, the token is drained from the buffer and returned. Otherwise, the
/// result of the closure is returned and nothing else happens.
pub fn parse<P, E>(buf: &mut EasyBuf, parseop: P) -> Poll<EasyBuf, E>
             where P: FnOnce(&mut Token) -> Poll<(), E> {
    let mut token = Token::new(buf);
    try_ready!(parseop(&mut token));
    Ok(Async::Ready(token.drain()))
}


/// Parses a token from a buffer and then converts it.
///
/// This starts out as `parse()`. If that returns either ready or with an
/// error, the result is given to the closure `convertop` which converts it
/// into whatever it likes.
pub fn convert<P, E, C, R, F>(buf: &mut EasyBuf, parseop: P, convertop: C)
                              -> Poll<R, F>
               where P: FnOnce(&mut Token) -> Poll<(), E>,
                     C: FnOnce(Result<&[u8], E>) -> Result<R, F> {
    // XXX Implementation with EasyBuf’s current limitations.
    let res = match try_result!(parse(buf, parseop)) {
        Ok(buf) => convertop(Ok(buf.as_slice())),
        Err(err) => convertop(Err(err))
    };
    res.map(|res| Async::Ready(res))
}

/// Skips over a token.
pub fn skip<P, E>(buf: &mut EasyBuf, parsef: P) -> Poll<(), E>
            where P: FnOnce(&mut Token) -> Poll<(), E> {
    // XXX Implementation with EasyBuf’s current limitations.
    try_ready!(parse(buf, parsef));
    Ok(Async::Ready(()))
}

/// Skips over an optional token.
///
/// If successful, returns whether there was a token or not.
pub fn skip_opt<P, E>(buf: &mut EasyBuf, parsef: P) -> Poll<bool, E>
                where P: FnOnce(&mut Token) -> Poll<(), E> {
    match try_result!(skip(buf, parsef)) {
        Ok(()) => Ok(Async::Ready(true)),
        Err(_) => Ok(Async::Ready(false))
    }
}


//============ Concrete Token Parsers ========================================

//------------ Specific Octets -----------------------------------------------

/// Expects the first octet of the token to be `value`.
///
/// If it is, advances over it. If it isn’t, returns an error.
pub fn octet(token: &mut Token, value: u8) -> Poll<(), TokenError> {
    let first = try_ready!(token.first());
    if first == value {
        token.advance(1);
        Ok(Async::Ready(()))
    }
    else {
        Err(TokenError)
    }
}

/// Advances the token if the first octet is `value`.
///
/// Returns whether it advanced or not.
pub fn opt_octet<E>(token: &mut Token, value: u8) -> Poll<bool, E> {
    let first = try_ready!(token.first());
    if first == value {
        token.advance(1);
        Ok(Async::Ready(true))
    }
    else {
        Ok(Async::Ready(false))
    }
}

/// Skips over the first octet in `buf` which must be `value`.
///
/// Returns an error if the first octet is anything else.
pub fn skip_octet(buf: &mut EasyBuf, value: u8) -> Poll<(), TokenError> {
    skip(buf, |token| octet(token, value))
}

/// Skips over the first octet in `buf` if it is `value`.
///
/// On success, returns whether it skipped an octet or not.
pub fn skip_opt_octet(buf: &mut EasyBuf, value: u8) -> Poll<bool, TokenError> {
    match try_result!(skip_octet(buf, value)) {
        Ok(()) => Ok(Async::Ready(true)),
        Err(_) => Ok(Async::Ready(false))
    }
}


//------------ Octet Categories ----------------------------------------------

/// Expects the first octet in `token` to meet `test`.
///
/// If the token is empty, returns non-ready. If `test` returns `true` for the
/// first octet in the token, advances over the octet and return ready. If
/// `test` returns `false`, return an error.
pub fn cat<O>(token: &mut Token, test: O) -> Poll<(), TokenError>
           where O: FnOnce(u8) -> bool {
    match try_ready!(token.advance_if(test)) {
        true => Ok(Async::Ready(())),
        false => Err(TokenError),
    }
}

/// Advances over a non-empty sequence of octets that meet `test`.
///
/// In order to decide whether the sequence is complete, this function always
/// needs at least one octet that does not meet `test`. It will return
/// non-ready if it can’t.
pub fn cats<O>(token: &mut Token, test: O) -> Poll<(), TokenError>
            where O: Fn(u8) -> bool {
    try_ready!(cat(token, |ch| test(ch)));
    try_ready!(opt_cats(token, |ch| test(ch)));
    Ok(Async::Ready(()))
}

/// Advances over a possibly empty sequence of octets that meet `test`.
///
/// In order to decide whether the sequence is complete, this function always
/// needs at least one octet that does not meet `test`. It will return
/// non-ready if it can’t.
/// Upon success, returns whether the sequence was non-empty.
pub fn opt_cats<O>(token: &mut Token, test: O) -> Poll<bool, TokenError>
                where O: Fn(u8) -> bool {
    if !try_ready!(token.advance_if(|ch| test(ch))) {
        return Ok(Async::Ready(false))
    }
    loop {
        if !try_ready!(token.advance_if(|ch| test(ch))) {
            return Ok(Async::Ready(false))
        }
    }
}


//------------ Literals ------------------------------------------------------

/// Advances a token over a literal. 
///
/// Note that in ABNF, literals are not case-sensitive. That is, the literal
/// `b"foo"` is matched also by `b"FoO"`.
///
/// If the token begins with the literal, the function will advance the
/// token by as many octets as `lit` and return ready. Unlike `cat()` and
/// friends, `literal()` will not wait for at least one more octet but
/// succeed right away if it finds the literal.
pub fn literal(token: &mut Token, lit: &[u8]) -> Poll<(), TokenError> {
    use std::cmp::min;
    use std::ascii::AsciiExt;

    let litlen = {
        let len = token.len();
        let litlen = lit.len();
        let minlen = min(len, litlen);
        let reduced = &token.as_slice()[..minlen];
        let litreduced = &lit[..minlen];

        if !reduced.eq_ignore_ascii_case(litreduced) {
            return Err(TokenError)
        }
        else if minlen < litlen {
            return Ok(Async::NotReady)
        }
        litlen
    };
    token.advance(litlen);
    Ok(Async::Ready(()))
}

/// Parse a literal from a buffer.
pub fn parse_literal(buf: &mut EasyBuf, lit: &[u8])
                     -> Poll<EasyBuf, TokenError> {
    parse(buf, |token| literal(token, lit))
}

/// Skip over a literal in a buffer.
pub fn skip_literal(buf: &mut EasyBuf, lit: &[u8]) -> Poll<(), TokenError> {
    skip(buf, |token| literal(token, lit))
}

/// If the buffer starts with `lit`, return `res`.
///
/// If there isn’t enough data to decide, returns non-ready. If the buffer
/// definitely doesn’t start with `lit`, returns an error.
///
/// This function can be used to construct an enum from literals:
///
/// ```
/// enum Command {
///     Echo,
///     Quit,
/// }
///
/// fn parse_command(buf: &mut EasyBuf) -> Poll<Command, TokenError> {
///     try_fail!(translate_literal(buf, "echo", Command::Echo));
///     try_fail!(translate_literal(buf, "quit", Command::Quit));
///     Err(TokenError)
/// }
/// ```
pub fn translate_literal<T>(buf: &mut EasyBuf, lit: &[u8], res: T)
                            -> Poll<T, TokenError> {
    try_ready!(skip_literal(buf, lit));
    Ok(Async::Ready(res))
}


//============ Errors ========================================================

/// An error happend while parsing a token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenError;

