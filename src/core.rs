//! Core Rules
//!
//! These are defined in RFC 5234, appendix B.1.

use ::{Async, EasyBuf, Poll};
use ::parse::token;
use ::parse::token::{TokenError, Token};


//------------ ALPHA ---------------------------------------------------------

pub fn test_alpha(ch: u8) -> bool {
    (ch >= 0x41 && ch <= 0x5A) || (ch >= 0x61 && ch <= 0x7A)
}

pub fn alpha(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_alpha)
}

pub fn alphas(token: &mut Token) -> Poll<(), TokenError> {
    token::cats(token, test_alpha)
}


//------------ BIT -----------------------------------------------------------

pub fn test_bit(ch: u8) -> bool {
    ch == b'0' || ch == b'1'
}

pub fn bit(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_bit)
}

pub fn bits(token: &mut Token) -> Poll<(), TokenError> {
    token::cats(token, test_bit)
}



//------------ NULL ----------------------------------------------------------

pub fn test_char(ch: u8) -> bool {
    ch > 0 && ch < 0x80
}

pub fn char(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_char)
}

pub fn chars(token: &mut Token) -> Poll<(), TokenError> {
    token::cats(token, test_char)
}


//------------ CR ------------------------------------------------------------

pub fn test_cr(ch: u8) -> bool {
    ch == 0x0D
}

pub fn cr(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_cr)
}


//------------ CRLF and lines terminated by CRLF -----------------------------

pub fn crlf(token: &mut Token) -> Poll<(), TokenError> {
    try_ready!(token.expect(test_cr, || TokenError));
    try_ready!(token.expect(test_lf, || TokenError));
    Ok(Async::Ready(()))
}

pub fn skip_crlf(buf: &mut EasyBuf) -> Poll<(), TokenError> {
    token::skip(buf, crlf)
}

pub fn line(token: &mut Token) -> Poll<(), TokenError> {
    let mut pos = None;
    for (i, slice) in token.as_slice().windows(2).enumerate() {
        if slice == b"\r\n" {
            pos = Some(i);
            break;
        }
    }
    match pos {
        Some(pos) => {
            token.advance(pos + 2);
            Ok(Async::Ready(()))
        }
        None => Ok(Async::NotReady)
    }
}

pub fn parse_line(buf: &mut EasyBuf) -> Poll<EasyBuf, TokenError> {
    token::parse(buf, line)
}

//------------ CTL -----------------------------------------------------------

pub fn test_ctl(ch: u8) -> bool {
    ch < 0x20 || ch == 0x7F
}

pub fn ctl(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_ctl)
}

pub fn ctls(token: &mut Token) -> Poll<(), TokenError> {
    token::cats(token, test_ctl)
}


//------------ DIGIT ---------------------------------------------------------

pub fn test_digit(ch: u8) -> bool {
    ch >= 0x30 && ch <= 0x39
}

pub fn digit(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_digit)
}

pub fn digits(token: &mut Token) -> Poll<(), TokenError> {
    token::cats(token, test_digit)
}

macro_rules! convert_uint {
    ( $token_name:ident, $uint:ty, $parsef:expr, $radix:expr) => {
        pub fn $token_name(buf: &mut EasyBuf) -> Poll<$uint, TokenError> {
            token::convert(buf, $parsef, |digits| {
                let digits = digits?;
                let mut res = 0 as $uint;
                for item in digits {
                    let x = (*item as char).to_digit($radix).unwrap() as $uint;
                    res = match res.checked_mul($radix) {
                        Some(x) => x,
                        None => return Err(TokenError)
                    };
                    res = match res.checked_add(x) {
                        Some(x) => x,
                        None => return Err(TokenError)
                    };
                }
                Ok(res)
            })
        }
    }
}

convert_uint!(u8_digits, u8, digits, 10);
convert_uint!(u16_digits, u16, digits, 10);
convert_uint!(u32_digits, u32, digits, 10);
convert_uint!(u64_digits, u64, digits, 10);


//------------ DQUOTE --------------------------------------------------------

pub fn test_dquote(ch: u8) -> bool {
    ch == b'"'
}

pub fn dquote(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_dquote)
}

pub fn skip_dquote(buf: &mut EasyBuf) -> Poll<(), TokenError> {
    token::skip(buf, dquote)
}


//------------ HEXDIG --------------------------------------------------------

pub fn test_hexdig(ch: u8) -> bool {
    (ch >= 0x30 && ch <= 0x39) || (ch >= 0x41 && ch <= 0x46)
        || (ch >= 0x61 && ch <= 0x66)
}

pub fn hexdig(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_hexdig)
}

pub fn hexdigs(token: &mut Token) -> Poll<(), TokenError> {
    token::cats(token, test_hexdig)
}

convert_uint!(u8_hexdigs, u8, hexdigs, 16);
convert_uint!(u16_hexdigs, u16, hexdigs, 16);
convert_uint!(u32_hexdigs, u32, hexdigs, 16);
convert_uint!(u64_hexdigs, u64, hexdigs, 16);


//------------ HTAB ----------------------------------------------------------

pub fn test_htab(ch: u8) -> bool {
    ch == 0x09
}

pub fn htab(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_htab)
}


//------------ LF ------------------------------------------------------------

pub fn test_lf(ch: u8) -> bool {
    ch == 0x0A
}

pub fn lf(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_lf)
}


//------------ LWSP ----------------------------------------------------------

pub fn lwsp(token: &mut Token) -> Poll<(), TokenError> {
        loop {
            if try_result!(wsp(token)).is_err()
                    || try_result!(crlf(token)).is_err() {
                return Ok(Async::Ready(()))
            }
        }
}

pub fn skip_lwsp(buf: &mut EasyBuf) -> Poll<(), TokenError> {
    token::skip(buf, lwsp)
}


//------------ SP ------------------------------------------------------------

pub fn test_sp(ch: u8) -> bool {
    ch == 0x20
}

pub fn sp(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_sp)
}

pub fn sps(token: &mut Token) -> Poll<(), TokenError> {
    token::cats(token, test_sp)
}


//------------ VCHAR ---------------------------------------------------------

pub fn test_vchar(ch: u8) -> bool {
    ch >= 0x21 && ch <= 0x7E
}

pub fn vchar(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_vchar)
}

pub fn vchars(token: &mut Token) -> Poll<(), TokenError> {
    token::cats(token, test_vchar)
}


//------------ WSP -----------------------------------------------------------

pub fn test_wsp(ch: u8) -> bool {
    ch == 0x20 || ch == 0x09
}

pub fn wsp(token: &mut Token) -> Poll<(), TokenError> {
    token::cat(token, test_wsp)
}

pub fn wsps(token: &mut Token) -> Poll<(), TokenError> {
    token::cats(token, test_wsp)
}

pub fn opt_wsps(token: &mut Token) -> Poll<bool, TokenError> {
    token::opt_cats(token, test_wsp)
}

pub fn skip_wsps(buf: &mut EasyBuf) -> Poll<(), TokenError> {
    token::skip(buf, wsps)
}

pub fn skip_opt_wsps(buf: &mut EasyBuf) -> Poll<bool, TokenError> {
    token::skip_opt(buf, wsps)
}


//============ Test =========================================================

#[cfg(test)]
mod test {
    use futures::Async;
    use tokio_core::io::EasyBuf;
    use super::*;

    fn buf(slice: &[u8]) -> EasyBuf { EasyBuf::from(Vec::from(slice)) }

    #[test]
    fn test_u8_digits() {
        for i in 0u8..255 {
            assert_eq!(u8_digits(&mut EasyBuf::from(format!("{} ", i)
                                                    .into_bytes())),
                       Ok(Async::Ready(i)));
        }
        assert!(u8_digits(&mut buf(b"256 ")).is_err());
        assert!(u8_digits(&mut buf(b"2568 ")).is_err());
        assert!(u8_digits(&mut buf(b"fee ")).is_err());
        assert!(u8_digits(&mut buf(b" ")).is_err());
    }

    #[test]
    fn test_u16_hexdigs() {
        for i in 0u16..0xFFFF {
            assert_eq!(u16_hexdigs(&mut EasyBuf::from(format!("{:x} ", i)
                                                        .into_bytes())),
                       Ok(Async::Ready(i)));
            assert_eq!(u16_hexdigs(&mut EasyBuf::from(format!("{:X} ", i)
                                                        .into_bytes())),
                       Ok(Async::Ready(i)));
        }
        assert!(u16_hexdigs(&mut buf(b"70256 ")).is_err());
        assert!(u16_hexdigs(&mut buf(b" ")).is_err());
    }
}
