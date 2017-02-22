
use std::net::{Ipv4Addr, Ipv6Addr};
use ::{Async, EasyBuf, Poll};
use ::parse::{rule, token};
use ::parse::token::TokenError;
use ::core::{u16_hexdigs, u8_digits};
 

//------------ parse_ipv4addr ------------------------------------------------

/// Parses an IPv4 address
pub fn parse_ipv4_addr(buf: &mut EasyBuf) -> Poll<Ipv4Addr, TokenError> {
    rule::group(buf, |buf| {
        let a = try_ready!(u8_digits(buf));
        try_ready!(token::skip_octet(buf, b'.'));
        let b = try_ready!(u8_digits(buf));
        try_ready!(token::skip_octet(buf, b'.'));
        let c = try_ready!(u8_digits(buf));
        try_ready!(token::skip_octet(buf, b'.'));
        let d = try_ready!(u8_digits(buf));
        Ok(Async::Ready(Ipv4Addr::new(a, b, c, d)))
    })
}


//------------ parse_ipv6addr ------------------------------------------------

/// Parses an IPv6 address
///
//  IPv6-addr      = IPv6-full / IPv6-comp / IPv6v4-full / IPv6v4-comp
//
pub fn parse_ipv6_addr(buf: &mut EasyBuf) -> Poll<Ipv6Addr, TokenError> {
    try_fail!(ipv6_full(buf));
    try_fail!(ipv6_comp(buf));
    try_fail!(ipv6v4_full(buf));
    try_fail!(ipv6v4_comp(buf));
    Err(TokenError)
}

//  IPv6-full      = IPv6-hex 7(":" IPv6-hex)
fn ipv6_full(buf: &mut EasyBuf) -> Poll<Ipv6Addr, TokenError> {
    rule::group(buf, |buf| {
        let a = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let b = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let c = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let d = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let e = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let f = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let g = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let h = try_ready!(u16_hexdigs(buf));
        Ok(Async::Ready(Ipv6Addr::new(a, b, c, d, e, f, g, h)))
    })
}

// IPv6-comp      = [IPv6-hex *5(":" IPv6-hex)] "::"
//                  [IPv6-hex *5(":" IPv6-hex)]
fn ipv6_comp(buf: &mut EasyBuf) -> Poll<Ipv6Addr, TokenError> {
    rule::group(buf, |buf| {
        let (mut left, left_count) = try_ready!(ipv6_comp_left(buf, 6));
        let (right, right_count) = try_ready!(ipv6_comp_right(buf,
                                                              6 - left_count));
        for i in 0..right_count {
            left[8 - right_count + i] = right[i];
        }
        Ok(Async::Ready(Ipv6Addr::new(left[0], left[1], left[2], left[3],
                                      left[4], left[5], left[6], left[7])))
    })
}

// IPv6v4-full    = IPv6-hex 5(":" IPv6-hex) ":" IPv4-address-literal
fn ipv6v4_full(buf: &mut EasyBuf) -> Poll<Ipv6Addr, TokenError> {
    rule::group(buf, |buf| {
        let a = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let b = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let c = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let d = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let e = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let f = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        let g1 = try_ready!(u8_digits(buf));
        try_ready!(token::skip_octet(buf, b'.'));
        let g2 = try_ready!(u8_digits(buf));
        try_ready!(token::skip_octet(buf, b'.'));
        let h1 = try_ready!(u8_digits(buf));
        try_ready!(token::skip_octet(buf, b'.'));
        let h2 = try_ready!(u8_digits(buf));
        Ok(Async::Ready(Ipv6Addr::new(a, b, c, d, e, f,
                                      (g1 as u16) << 8 | (g2 as u16),
                                      (h1 as u16) << 8 | (h2 as u16))))
    })
}

// IPv6v4-comp    = [IPv6-hex *3(":" IPv6-hex)] "::"
//                  [IPv6-hex *3(":" IPv6-hex) ":"]
//                  IPv4-address-literal
fn ipv6v4_comp(buf: &mut EasyBuf) -> Poll<Ipv6Addr, TokenError> {
    rule::group(buf, |buf| {
        let (mut left, left_count) = try_ready!(ipv6_comp_left(buf, 4));
        let (right, right_count) = try_ready!(ipv6_comp_right(buf,
                                                              4 - left_count));
        let v4 = try_ready!(parse_ipv4_addr(buf));
        let v4 = v4.octets();
        for i in 0..right_count {
            left[6 - right_count + 1] = right[i];
        }
        left[6] = (v4[0] as u16) << 8 | (v4[1] as u16);
        left[7] = (v4[2] as u16) << 8 | (v4[3] as u16);
        Ok(Async::Ready(Ipv6Addr::new(left[0], left[1], left[2], left[3],
                                      left[4], left[5], left[6], left[7])))
    })
}

/// Parses the left hand side of a compressed IPv6 address.
///
/// Returns the parsed components and the number of them.
fn ipv6_comp_left(buf: &mut EasyBuf, max: usize)
                  -> Poll<([u16; 8], usize), TokenError> {
    let mut res = [0u16, 0, 0, 0, 0, 0, 0, 0];

    // Minimum size is two: b"::" or b"0:"
    if buf.len() < 3 { return Ok(Async::NotReady) }

    // We may start with two colons, in which case there is no left hand
    // side.
    if buf.as_slice()[0] == b':' && buf.as_slice()[1] == b':' {
        buf.drain_to(2);
        return Ok(Async::Ready((res, 0)));
    }

    // Up to six components that end in a colon and may end in a
    // double colon
    for i in 0..max {
        let v = try_ready!(u16_hexdigs(buf));
        try_ready!(token::skip_octet(buf, b':'));
        res[i] = v;
        if buf.as_slice().first() == Some(&b':') {
            buf.drain_to(1);
            return Ok(Async::Ready((res, i + 1)))
        }
    }

    Ok(Async::Ready((res, max)))
}

/// Parses the right hand side of a compressed IPv6 address.
///
/// Returns the parsed components and the number of them.
fn ipv6_comp_right(buf: &mut EasyBuf, max: usize)
                   -> Poll<([u16; 8], usize), TokenError> {
    let mut res = [0u16, 0, 0, 0, 0, 0, 0, 0];

    for i in 0..max {
        match u16_hexdigs(buf) {
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Err(_) => {
                if i == 0 {
                    return Ok(Async::Ready((res, 0)))
                }
                else {
                    return Err(TokenError)
                }
            }
            Ok(Async::Ready(v)) => {
                res[i] = v;
            }
        }
        match token::skip_octet(buf, b':') {
            Ok(Async::Ready(_)) => {
                if i == max - 1 {
                    break;
                }
            }
            _ => { return Ok(Async::Ready((res, i + 1))); }
        }
    }
    Ok(Async::Ready((res, max)))
}


//============ Test =========================================================

#[cfg(test)]
mod test {
    use std::net::{Ipv4Addr, Ipv6Addr};
    use futures::Async;
    use tokio_core::io::EasyBuf;
    use super::*;

    fn buf(slice: &[u8]) -> EasyBuf { EasyBuf::from(Vec::from(slice)) }

    #[test]
    fn ipv4_good() {
        assert_eq!(parse_ipv4_addr(&mut buf(b"127.0.0.1 ")),
                   Ok(Async::Ready(Ipv4Addr::new(127, 0, 0, 1))));
    }

    #[test]
    fn ipv6_good() {
        assert_eq!(
            parse_ipv6_addr(
                &mut buf(b"FEDC:BA98:7654:3210:FEDC:BA98:7654:3210 ")
            ),
            Ok(Async::Ready(Ipv6Addr::new(0xFEDC, 0xBA98, 0x7654,
                                          0x3210, 0xFEDC, 0xBA98,
                                          0x7654, 0x3210)))
        );
        assert_eq!(
            parse_ipv6_addr(&mut buf(b"1080:0:0:0:8:800:200C:417A ")),
            Ok(Async::Ready(Ipv6Addr::new(0x1080, 0, 0, 0,
                                          8, 0x800, 0x200C, 0x417A)))
        );
        assert_eq!(
            parse_ipv6_addr(&mut buf(b"1080::8:800:200C:417A ")),
            Ok(Async::Ready(Ipv6Addr::new(0x1080, 0, 0, 0,
                                          8, 0x800, 0x200C, 0x417A)))
        );
        assert_eq!(
            parse_ipv6_addr(&mut buf(b"FF01::43 ")),
            Ok(Async::Ready(Ipv6Addr::new(0xFF01, 0, 0, 0, 0, 0, 0, 0x43)))
        );
        assert_eq!(
            parse_ipv6_addr(&mut buf(b"::1 ")),
            Ok(Async::Ready(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)))
        );
        assert_eq!(
            parse_ipv6_addr(&mut buf(b":: ")),
            Ok(Async::Ready(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)))
        );
    }
}
