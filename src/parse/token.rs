/// Token Parsing
///

use std::ops;
use ::{Async, EasyBuf, Poll};


//============ Basic Token Parsing ===========================================

//------------ Token --------------------------------------------------------

pub struct Token<'a> {
    buf: &'a mut EasyBuf,
    start: usize
}


impl<'a> Token<'a> {
    pub fn new(buf: &'a mut EasyBuf) -> Self {
        Token {
            buf: buf,
            start: 0
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf.as_slice()[self.start..]
    }

    pub fn advance(&mut self, at: usize) {
        assert!(self.start + at <= self.buf.len());
        self.start += at;
    }

    /// Advances one octet if `test` returned `true` for it.
    ///
    /// Returns whether it advanced.
    pub fn advance_if<F, E>(&mut self, test: F) -> Poll<bool, E>
                      where F: FnOnce(u8) -> bool {
        let res = test(try_ready!(self.first()));
        if res {
            self.advance(1)
        }
        Ok(Async::Ready(res))
    }

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

    pub fn first<E>(&self) -> Poll<u8, E> {
        if self.start < self.buf.len() {
            Ok(Async::Ready(self.buf.as_slice()[self.start]))
        }
        else {
            Ok(Async::NotReady)
        }
    }

    pub fn drain(self) -> EasyBuf {
        self.buf.drain_to(self.start)
    }

    pub fn skip(self) {
        let _  = self.buf.drain_to(self.start);
    }
}

impl<'a> ops::Deref for Token<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}


//------------ Essential Token Parsing Functions -----------------------------

/// Parses a token.
pub fn parse<P, E>(buf: &mut EasyBuf, parsef: P) -> Poll<EasyBuf, E>
             where P: FnOnce(&mut Token) -> Poll<(), E> {
    let mut token = Token::new(buf);
    try_ready!(parsef(&mut token));
    Ok(Async::Ready(token.drain()))
}


/// Parses and then converts a token.
pub fn convert<P, E, C, R, F>(buf: &mut EasyBuf, parsef: P, convertf: C)
                              -> Poll<R, F>
               where P: FnOnce(&mut Token) -> Poll<(), E>,
                     C: FnOnce(Result<&[u8], E>) -> Result<R, F> {
    // XXX Implementation with EasyBuf’s current limitations.
    let res = match try_result!(parse(buf, parsef)) {
        Ok(buf) => convertf(Ok(buf.as_slice())),
        Err(err) => convertf(Err(err))
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
pub fn skip_opt<P, E>(buf: &mut EasyBuf, parsef: P) -> Poll<bool, E>
                where P: FnOnce(&mut Token) -> Poll<(), E> {
    match try_result!(skip(buf, parsef)) {
        Ok(()) => Ok(Async::Ready(true)),
        Err(_) => Ok(Async::Ready(false))
    }
}


//============ Concrete Token Parsers ========================================

//------------ Specific Octets -----------------------------------------------

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


pub fn skip_octet(buf: &mut EasyBuf, value: u8) -> Poll<(), TokenError> {
    skip(buf, |token| octet(token, value))
}


pub fn skip_opt_octet(buf: &mut EasyBuf, value: u8) -> Poll<bool, TokenError> {
    match try_result!(skip_octet(buf, value)) {
        Ok(()) => Ok(Async::Ready(true)),
        Err(_) => Ok(Async::Ready(false))
    }
}


//------------ Octet Categories ----------------------------------------------

pub fn cat<O>(token: &mut Token, test: O) -> Poll<(), TokenError>
           where O: FnOnce(u8) -> bool {
    match try_ready!(token.advance_if(test)) {
        true => Ok(Async::Ready(())),
        false => Err(TokenError),
    }
}


pub fn cats<O>(token: &mut Token, test: O) -> Poll<(), TokenError>
            where O: Fn(u8) -> bool {
    try_ready!(cat(token, |ch| test(ch)));
    try_ready!(opt_cats(token, |ch| test(ch)));
    Ok(Async::Ready(()))
}


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


pub fn parse_literal(buf: &mut EasyBuf, lit: &[u8])
                     -> Poll<EasyBuf, TokenError> {
    parse(buf, |token| literal(token, lit))
}

pub fn skip_literal(buf: &mut EasyBuf, lit: &[u8]) -> Poll<(), TokenError> {
    skip(buf, |token| literal(token, lit))
}

pub fn translate_literal<T>(buf: &mut EasyBuf, lit: &[u8], res: T)
                            -> Poll<T, TokenError> {
    try_ready!(skip_literal(buf, lit));
    Ok(Async::Ready(res))
}


//============ Errors ========================================================

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenError;

