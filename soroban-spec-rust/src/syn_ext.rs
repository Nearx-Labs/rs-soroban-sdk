use std::collections::{HashMap, HashSet};

use proc_macro2::Ident;
use stellar_xdr::{ScSpecEntry, ScSymbol, StringM, SC_SPEC_TYPE_NAME_LIMIT};

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
    str_to_ident(last_segment(&s))
}

/// Returns the type's own name from a qualified spec name, i.e. everything after
/// the last `::`.
fn last_segment(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// The identifier each of a spec's user-defined types is named by in generated
/// code.
///
/// A type's name in the spec qualifies it with the module it was defined in, and
/// generated code names the type by only the last segment of that path. Two
/// types defined in different modules can end in the same segment, so the first
/// type to claim a segment keeps it and each one after it gets a number
/// appended, in the order the entries appear in the spec:
///
/// | spec name                 | generated  |
/// |---------------------------|------------|
/// | `mycrate::MyError`        | `MyError`  |
/// | `mycrate::mymod::MyError` | `MyError2` |
///
/// A type whose last segment nothing else shares always keeps it, and the same
/// spec always produces the same identifiers.
#[derive(Debug, Default)]
pub struct TypeNames {
    /// Qualified spec name to the identifier generated code names it by. Holds
    /// only the types whose identifier differs from their last segment, so an
    /// absent name resolves to its last segment.
    renamed: HashMap<String, String>,
}

impl TypeNames {
    /// Assigns an identifier to every user-defined type the entries define.
    pub fn from_specs(specs: &[ScSpecEntry]) -> Result<Self, GenerateError> {
        // The distinct names the entries define, in the order they appear, so
        // that the same spec always produces the same identifiers. A name can
        // appear more than once, and each time refers to the same type.
        let mut defined = Vec::new();
        let mut seen = HashSet::new();
        for name in specs.iter().filter_map(udt_def_name) {
            let name = name.to_utf8()?;
            if seen.insert(name.clone()) {
                defined.push(name);
            }
        }

        // The first type to claim a last segment keeps it, so a type only ever
        // loses its own name to one that came before it, never to a number
        // handed to a type that collided with something else.
        let mut taken = HashSet::new();
        let colliding: Vec<&String> = defined
            .iter()
            .filter(|name| !taken.insert(last_segment(name).to_string()))
            .collect();

        let mut renamed = HashMap::new();
        for name in colliding {
            let base = last_segment(name);
            let mut n = 1u32;
            let ident = loop {
                n += 1;
                let ident = format!("{base}{n}");
                if taken.insert(ident.clone()) {
                    break ident;
                }
            };
            renamed.insert(name.clone(), ident);
        }
        Ok(Self { renamed })
    }

    /// Creates the Rust identifier naming the user-defined type that the given
    /// qualified spec name refers to.
    ///
    /// A name this was not built from resolves to its own last segment, which is
    /// what a spec containing only that one type would produce.
    pub fn ident(&self, name: &(impl ToUtf8 + ?Sized)) -> Result<Ident, GenerateError> {
        let name = name.to_utf8()?;
        match self.renamed.get(&name) {
            Some(ident) => str_to_ident(ident.as_str()),
            None => str_to_ident(last_segment(&name)),
        }
    }
}

/// Returns the name of the user-defined type an entry defines, or `None` for
/// entries that do not define one.
fn udt_def_name(entry: &ScSpecEntry) -> Option<&StringM<{ SC_SPEC_TYPE_NAME_LIMIT as u32 }>> {
    match entry {
        ScSpecEntry::UdtStructV0(s) => Some(&s.name),
        ScSpecEntry::UdtUnionV0(u) => Some(&u.name),
        ScSpecEntry::UdtEnumV0(e) => Some(&e.name),
        ScSpecEntry::UdtErrorEnumV0(e) => Some(&e.name),
        // An event is named by a symbol rather than a qualified type name, and
        // is never referred to as a type.
        ScSpecEntry::EventV0(_) | ScSpecEntry::FunctionV0(_) => None,
    }
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

#[cfg(test)]
mod test {
    use super::TypeNames;
    use stellar_xdr::{
        ScSpecEntry, ScSpecEventDataFormat, ScSpecEventV0, ScSpecUdtErrorEnumV0, ScSpecUdtStructV0,
        VecM,
    };

    fn error_enum(name: &str) -> ScSpecEntry {
        ScSpecEntry::UdtErrorEnumV0(ScSpecUdtErrorEnumV0 {
            doc: "".try_into().unwrap(),
            lib: "".try_into().unwrap(),
            name: name.try_into().unwrap(),
            cases: VecM::default(),
        })
    }

    fn struct_(name: &str) -> ScSpecEntry {
        ScSpecEntry::UdtStructV0(ScSpecUdtStructV0 {
            doc: "".try_into().unwrap(),
            lib: "".try_into().unwrap(),
            name: name.try_into().unwrap(),
            fields: VecM::default(),
        })
    }

    fn event(name: &str) -> ScSpecEntry {
        ScSpecEntry::EventV0(ScSpecEventV0 {
            doc: "".try_into().unwrap(),
            lib: "".try_into().unwrap(),
            name: name.try_into().unwrap(),
            prefix_topics: VecM::default(),
            params: VecM::default(),
            data_format: ScSpecEventDataFormat::Map,
        })
    }

    fn ident(names: &TypeNames, name: &str) -> String {
        names.ident(name).unwrap().to_string()
    }

    #[test]
    fn test_unique_name_is_its_last_segment() {
        let specs = [error_enum("mycrate::mymod::MyError")];
        let names = TypeNames::from_specs(&specs).unwrap();
        assert_eq!(ident(&names, "mycrate::mymod::MyError"), "MyError");
    }

    #[test]
    fn test_unqualified_name_is_itself() {
        let specs = [error_enum("MyError")];
        let names = TypeNames::from_specs(&specs).unwrap();
        assert_eq!(ident(&names, "MyError"), "MyError");
    }

    #[test]
    fn test_colliding_names_are_numbered_in_spec_order() {
        let specs = [
            error_enum("mycrate::MyError"),
            error_enum("mycrate::mymod::MyError"),
        ];
        let names = TypeNames::from_specs(&specs).unwrap();
        assert_eq!(ident(&names, "mycrate::MyError"), "MyError");
        assert_eq!(ident(&names, "mycrate::mymod::MyError"), "MyError2");
    }

    #[test]
    fn test_third_collision_continues_the_numbering() {
        let specs = [
            error_enum("a::MyError"),
            error_enum("b::MyError"),
            error_enum("c::MyError"),
        ];
        let names = TypeNames::from_specs(&specs).unwrap();
        assert_eq!(ident(&names, "a::MyError"), "MyError");
        assert_eq!(ident(&names, "b::MyError"), "MyError2");
        assert_eq!(ident(&names, "c::MyError"), "MyError3");
    }

    #[test]
    fn test_numbering_skips_an_identifier_another_type_holds() {
        // The type actually named MyError2 claims that identifier first, so the
        // second MyError has to skip past it.
        let specs = [
            error_enum("a::MyError"),
            error_enum("c::MyError2"),
            error_enum("b::MyError"),
        ];
        let names = TypeNames::from_specs(&specs).unwrap();
        assert_eq!(ident(&names, "a::MyError"), "MyError");
        assert_eq!(ident(&names, "c::MyError2"), "MyError2");
        assert_eq!(ident(&names, "b::MyError"), "MyError3");
    }

    #[test]
    fn test_numbering_skips_an_identifier_a_later_type_holds() {
        // The type actually named MyError2 keeps that name even though it is
        // declared after the collision that would otherwise be numbered into it.
        let specs = [
            error_enum("a::MyError"),
            error_enum("b::MyError"),
            error_enum("c::MyError2"),
        ];
        let names = TypeNames::from_specs(&specs).unwrap();
        assert_eq!(ident(&names, "a::MyError"), "MyError");
        assert_eq!(ident(&names, "b::MyError"), "MyError3");
        assert_eq!(ident(&names, "c::MyError2"), "MyError2");
    }

    #[test]
    fn test_types_of_different_kinds_collide() {
        let specs = [struct_("a::Thing"), error_enum("b::Thing")];
        let names = TypeNames::from_specs(&specs).unwrap();
        assert_eq!(ident(&names, "a::Thing"), "Thing");
        assert_eq!(ident(&names, "b::Thing"), "Thing2");
    }

    #[test]
    fn test_repeated_name_keeps_its_first_identifier() {
        let specs = [error_enum("a::MyError"), error_enum("a::MyError")];
        let names = TypeNames::from_specs(&specs).unwrap();
        assert_eq!(ident(&names, "a::MyError"), "MyError");
    }

    #[test]
    fn test_events_are_not_named_as_types() {
        // An event is named by a symbol, not a qualified type name, so it takes
        // no part in naming types.
        let specs = [event("Thing"), error_enum("a::Thing")];
        let names = TypeNames::from_specs(&specs).unwrap();
        assert_eq!(ident(&names, "a::Thing"), "Thing");
    }

    #[test]
    fn test_name_the_spec_does_not_define_is_its_last_segment() {
        let names = TypeNames::default();
        assert_eq!(ident(&names, "a::b::Thing"), "Thing");
        assert_eq!(ident(&names, "Thing"), "Thing");
    }
}
