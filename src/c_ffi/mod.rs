//! C foreign-function-interface helpers.
//!
//! This module provides:
//! - [`CType`]           — C primitive types we can bridge
//! - [`CFuncSig`]        — a parsed C function signature (name + param/return types)
//! - [`CFunc`]           — a loaded, callable C function (library handle + pointer + sig)
//! - [`parse_c_header`]  — parse a C header via the system C preprocessor + lang-c
//! - [`call_c_function`] — call a [`CFunc`] from a [`DataValue`] argument
//! - [`call_cfuncsig`]   — call a [`CFuncSig`] via the pre-loaded library registry

mod types;
mod parse;
mod call;

pub use types::{CType, CFuncSig, CFunc};
pub use parse::parse_c_header;
pub use call::{call_c_function, call_cfuncsig, open_library};
