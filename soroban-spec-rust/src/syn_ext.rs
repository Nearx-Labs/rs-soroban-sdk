use proc_macro2::Ident;
use stellar_xdr::{ScSymbol, StringM};

use crate::types::GenerateError;

pub trait IntoIdent {
    fn into_ident(&self) -> Result<Ident, GenerateError>;
}

impl IntoIdent for str {
    fn into_ident(&self) -> Result<Ident, GenerateError> {
        syn::parse_str::<Ident>(self).map_err(|_| GenerateError::InvalidIdent(self.to_string()))
    }
}

impl<const N: u32> IntoIdent for StringM<N> {
    fn into_ident(&self) -> Result<Ident, GenerateError> {
        let s = self
            .to_utf8_string()
            .map_err(|_| GenerateError::InvalidUtf8)?;
        s.as_str().into_ident()
    }
}

impl IntoIdent for ScSymbol {
    fn into_ident(&self) -> Result<Ident, GenerateError> {
        self.0.into_ident()
    }
}

/// Creates a Rust identifier from a string or spec name, returning an error if
/// it contains invalid UTF-8 or is not a valid identifier.
pub fn str_to_ident(s: &(impl IntoIdent + ?Sized)) -> Result<Ident, GenerateError> {
    s.into_ident()
}

/// Creates a Rust identifier naming a user-defined type, from the qualified
/// name the spec knows it by.
///
/// A type's name in the spec qualifies it with the module it was defined in, so
/// only its last segment is the type's own name, and only that segment is a
/// valid identifier.
pub fn type_name_to_ident(s: &(impl ToUtf8 + ?Sized)) -> Result<Ident, GenerateError> {
    let s = s.to_utf8()?;
    let name = s.rsplit("::").next().unwrap_or(&s);
    str_to_ident(name)
}

pub trait ToUtf8 {
    fn to_utf8(&self) -> Result<String, GenerateError>;
}

impl ToUtf8 for str {
    fn to_utf8(&self) -> Result<String, GenerateError> {
        Ok(self.to_string())
    }
}

impl<const N: u32> ToUtf8 for StringM<N> {
    fn to_utf8(&self) -> Result<String, GenerateError> {
        self.to_utf8_string()
            .map_err(|_| GenerateError::InvalidUtf8)
    }
}
