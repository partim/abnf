//! Core Rules
//!
//! These are defined in RFC 5234, appendix B.1.
//!

use futures::Async;
use tokio_core::io::EasyBuf;
use super::parse;


//------------ ALPHA ---------------------------------------------------------

pub fn test_alpha(ch: u8) -> bool {
    (ch >= 0x41 && ch <= 0x5A) || (ch >= 0x61 && ch <= 0x7A)
}

pub fn alpha(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_alpha)
}

pub fn alphas(buf: &mut EasyBuf) -> parse::Poll<EasyBuf> {
    parse::cats(buf, test_alpha)
}

/// Parses a final `1*ALPHA`.
pub fn final_alphas(buf: &mut EasyBuf) -> parse::Result<EasyBuf> {
    parse::final_cats(buf, test_alpha)
}


//------------ BIT -----------------------------------------------------------

pub fn test_bit(ch: u8) -> bool {
    ch == b'0' || ch == b'1'
}

pub fn bit(buf: &mut EasyBuf) -> parse::Poll<bool> {
    Ok(Async::Ready(try_ready!(parse::cat(buf, test_bit)) == b'1'))
}

pub fn bits(buf: &mut EasyBuf) -> parse::Poll<EasyBuf> {
    parse::cats(buf, test_bit)
}

pub fn final_bits(buf: &mut EasyBuf) -> parse::Result<EasyBuf> {
    parse::final_cats(buf, test_bit)
}


//------------ NULL ----------------------------------------------------------

pub fn test_char(ch: u8) -> bool {
    ch > 0 && ch < 0x80
}

pub fn char(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_char)
}

pub fn chars(buf: &mut EasyBuf) -> parse::Poll<EasyBuf> {
    parse::cats(buf, test_char)
}

pub fn final_chars(buf: &mut EasyBuf) -> parse::Result<EasyBuf> {
    parse::final_cats(buf, test_char)
}


//------------ CR ------------------------------------------------------------

pub fn test_cr(ch: u8) -> bool {
    ch == 0x0D
}

pub fn cr(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_cr)
}


//------------ CRLF ----------------------------------------------------------

pub fn crlf(buf: &mut EasyBuf) -> parse::Poll<()> {
    match buf.len() {
        0 => Ok(Async::NotReady),
        1 => {
            if test_cr(buf.as_slice()[0]) { Ok(Async::NotReady) }
            else { Err(parse::Error) }
        }
        _ => {
            if test_cr(buf.as_slice()[0]) && test_lf(buf.as_slice()[1]) {
                buf.drain_to(2);
                Ok(Async::Ready(()))
            }
            else {
                Err(parse::Error)
            }
        }
    }
}


//------------ CTL -----------------------------------------------------------

pub fn test_ctl(ch: u8) -> bool {
    ch < 0x20 || ch == 0x7F
}

pub fn ctl(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_ctl)
}

pub fn ctls(buf: &mut EasyBuf) -> parse::Poll<EasyBuf> {
    parse::cats(buf, test_ctl)
}

pub fn final_ctls(buf: &mut EasyBuf) -> parse::Result<EasyBuf> {
    parse::final_cats(buf, test_ctl)
}


//------------ DIGIT ---------------------------------------------------------

pub fn test_digit(ch: u8) -> bool {
    ch >= 0x30 && ch <= 0x39
}

pub fn digit(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_digit)
}

pub fn digits(buf: &mut EasyBuf) -> parse::Poll<EasyBuf> {
    parse::cats(buf, test_digit)
}

pub fn u8_digits(buf: &mut EasyBuf) -> parse::Poll<u8> {
    let res = try_ready!(digits(buf));
    let res = res.as_slice();
    match res.len() {
        1 => {
            Ok(Async::Ready(res[0] - b'0'))
        }
        2 => {
            Ok(Async::Ready((res[0] - b'0') * 10 + res[1] - b'0'))
        }
        3 => {
            let res = (((res[0] - b'0') as u16) * 100)
                    + (((res[1] - b'0') as u16) * 10)
                    + (res[2] - b'0') as u16;
            if res > 255 {
                Err(parse::Error)
            }
            else {
                Ok(Async::Ready(res as u8))
            }
        }
        _ => Err(parse::Error)
    }
}

pub fn final_digits(buf: &mut EasyBuf) -> parse::Result<EasyBuf> {
    parse::final_cats(buf, test_digit)
}


//------------ DQUOTE --------------------------------------------------------

pub fn test_dquote(ch: u8) -> bool {
    ch == b'"'
}

pub fn dquote(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_dquote)
}


//------------ HEXDIG --------------------------------------------------------

pub fn test_hexdig(ch: u8) -> bool {
    (ch >= 0x30 && ch <= 0x39) || (ch >= 0x41 && ch <= 0x46)
        || (ch >= 0x61 && ch <= 0x66)
}

pub fn translate_hexdig(ch: u8) -> u8 {
    if ch >= 0x30 && ch <= 0x39 {
        ch - b'0'
    }
    else if ch >= 0x41 && ch <= 0x46 {
        ch - b'A' + 10
    }
    else if ch >= 0x61 && ch <= 0x66 {
        ch - b'a' + 10
    }
    else {
        panic!("not a hexdig");
    }
}

pub fn hexdig(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_hexdig)
}

pub fn hexdigs(buf: &mut EasyBuf) -> parse::Poll<EasyBuf> {
    parse::cats(buf, test_hexdig)
}

pub fn u16_hexdigs(buf: &mut EasyBuf) -> parse::Poll<u16> {
    let res = try_ready!(hexdigs(buf));
    let res = res.as_slice();
    match res.len() {
        1 => {
            Ok(Async::Ready(translate_hexdig(res[0]) as u16))
        }
        2 => {
            Ok(Async::Ready((translate_hexdig(res[0]) as u16) << 4 |
                            translate_hexdig(res[1]) as u16))
        }
        3 => {
            Ok(Async::Ready((translate_hexdig(res[0]) as u16) << 8 |
                            (translate_hexdig(res[1]) as u16) << 4 |
                            translate_hexdig(res[2]) as u16))
        }
        4 => {
            Ok(Async::Ready((translate_hexdig(res[0]) as u16) << 12 |
                            (translate_hexdig(res[1]) as u16) << 8 |
                            (translate_hexdig(res[2]) as u16) << 4 |
                            translate_hexdig(res[3]) as u16))
        }
        _ => Err(parse::Error)
    }
}


pub fn final_hexdig(buf: &mut EasyBuf) -> parse::Result<EasyBuf> {
    parse::final_cats(buf, test_hexdig)
}


//------------ HTAB ----------------------------------------------------------

pub fn test_htab(ch: u8) -> bool {
    ch == 0x09
}

pub fn htab(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_htab)
}


//------------ LF ------------------------------------------------------------

pub fn test_lf(ch: u8) -> bool {
    ch == 0x0A
}

pub fn lf(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_lf)
}


//------------ LWSP ----------------------------------------------------------

// Non-final LWSP.
pub fn lwsp(_buf: &mut EasyBuf) -> parse::Poll<u8> {
    unimplemented!()
}

// Final LWSP.
pub fn final_lwsp(_buf: &mut EasyBuf) -> parse::Poll<u8> {
    unimplemented!()
}


//------------ SP ------------------------------------------------------------

pub fn test_sp(ch: u8) -> bool {
    ch == 0x20
}

pub fn sp(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_sp)
}

pub fn sps(buf: &mut EasyBuf) -> parse::Poll<EasyBuf> {
    parse::cats(buf, test_sp)
}

pub fn final_sps(buf: &mut EasyBuf) -> parse::Result<EasyBuf> {
    parse::final_cats(buf, test_sp)
}


//------------ VCHAR ---------------------------------------------------------

pub fn test_vchar(ch: u8) -> bool {
    ch >= 0x21 && ch <= 0x7E
}

pub fn vchar(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_vchar)
}

pub fn vchars(buf: &mut EasyBuf) -> parse::Poll<EasyBuf> {
    parse::cats(buf, test_vchar)
}

pub fn final_vchars(buf: &mut EasyBuf) -> parse::Result<EasyBuf> {
    parse::final_cats(buf, test_vchar)
}


//------------ WSP -----------------------------------------------------------

pub fn test_wsp(ch: u8) -> bool {
    ch == 0x20 || ch == 0x09
}

pub fn wsp(buf: &mut EasyBuf) -> parse::Poll<u8> {
    parse::cat(buf, test_wsp)
}

pub fn wsps(buf: &mut EasyBuf) -> parse::Poll<EasyBuf> {
    parse::cats(buf, test_wsp)
}

pub fn final_wsps(buf: &mut EasyBuf) -> parse::Result<EasyBuf> {
    parse::final_cats(buf, test_wsp)
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
