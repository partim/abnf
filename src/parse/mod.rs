//! Parsing.
//!
//! # Rules and Tokens
//!
//! ABNF only defines rules without giving meaning to what it is they parse.
//! But we need to do that. For this purpose we shall distinguish between
//! *rules* and *tokens*. A token shall be a sequence of octets taken from
//! the input buffer whereas rules describe how to combine tokens and other
//! rules into higher-level structures.
//!
//! We have different methods for parsing tokens and rules. This is because
//! the two behave differently when parsing. For tokens, we need to go over
//! the the input buffer and apply ABNF rules to find the end of the token,
//! and only then take the whole part to constuct the token from. With rules,
//! we can drain every matched rule from the buffer right away.
//!
//!
//! # Conventions
//!
//! Since quite a few functions in here are heavy on generic types, here are
//! a few convention to be followed. First, types are designated as follows:
//!
//! * `P` and `Q` are parsing closures,
//! * `C` and `D` are converting closures,
//! * `O` is a octet test closure (used by the cat family of token functions),
//! * `T` and `U` are types returned on success, and
//! * `E` and `F` are error types.
//!
//! Type arguments to functions are ordered such that for each closure
//! appearing in the argument list, the closure type is given first, then its
//! success type, then its error type following the order of closures and
//! leaving out repeat types for later closures.


pub mod rule;
pub mod token;
