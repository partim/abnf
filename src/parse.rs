
use std::result;
use futures;
use futures::Async;
use tokio_core::io::EasyBuf;


#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Error;


pub type Result<T> = result::Result<T, Error>;
pub type Poll<T> = futures::Poll<T, Error>;


//------------ Combinator-y Things -------------------------------------------

pub fn try<F, T>(buf: &mut EasyBuf, op: F) -> Poll<T>
           where F: FnOnce(&mut EasyBuf) -> Poll<T> {
    let orig_buf = buf.clone();
    let res = op(buf);
    match res {
        Ok(Async::NotReady) | Err(_) => *buf = orig_buf,
        _ => {}
    }
    res
}

pub fn repeat<F>(buf: &mut EasyBuf, op: F) -> Poll<EasyBuf>
              where F: Fn(&[u8]) -> Poll<Option<usize>> {
    let len = try_ready!(repeat_len(buf.as_slice(), op));
    Ok(Async::Ready(buf.drain_to(len)))
}

pub fn repeat_len<F>(slice: &[u8], op: F) -> Poll<usize>
                  where F: Fn(&[u8]) -> Poll<Option<usize>> {
    let mut len = 0;
    loop {
        match try_ready!(op(&slice[len..])) {
            Some(add) => len += add,
            None => return Ok(Async::Ready(len))
        }
    }
}


//------------ Various Parsing -----------------------------------------------

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
pub fn cat<F>(buf: &mut EasyBuf, test: F) -> Poll<u8>
           where F: FnOnce(u8) -> bool {
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
}
