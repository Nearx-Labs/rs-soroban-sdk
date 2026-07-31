//! Demonstrates how `#[contracttype]` works on a struct with a field whose type
//! is a struct imported into scope under an alias, i.e. `use path::Type as
//! Renamed;`.
//!
//! - `test_functional` and `test_functional_with_original_type` confirm that,
//!   used directly within this crate (not via a client generated from the
//!   contract spec), the generated type round-trips through a contract call
//!   correctly, whether values are constructed using the aliased name
//!   (`Renamed`) or the type's original name (`inner::Inner`) — they are the
//!   same type, so Rust's own type checking is what makes this work.
//! - `test_spec` verifies that aliased imports resolve to the referenced
//!   type's own spec identity. Field UDT references are not taken from the
//!   identifier written at the field-declaration site (`Renamed`); they are
//!   resolved through the type system to the referenced type's fully
//!   qualified definition path (`...::inner::Inner`), which is also the name
//!   the referenced type's own `UdtStructV0` entry is generated under. The
//!   alias never appears in the spec, so a client regenerated from the spec
//!   (e.g. `contractimport!` in another crate) sees a reference that matches
//!   an entry exactly, even though the field was declared via an alias.

use crate::{self as soroban_sdk};
use soroban_sdk::{contract, contractimpl, contracttype, Env};
use stellar_xdr::{
    Limits, ReadXdr, ScSpecEntry, ScSpecTypeDef, ScSpecTypeUdt, ScSpecUdtStructFieldV0,
    ScSpecUdtStructV0,
};

mod inner {
    use crate::{self as soroban_sdk};
    use soroban_sdk::contracttype;

    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    #[contracttype]
    pub struct Inner {
        pub a: i32,
        pub b: i32,
    }
}

use inner::Inner as Renamed;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Outer {
    pub inner: Renamed,
    pub c: i32,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn add(a: Outer, b: Outer) -> (Outer, Outer) {
        (a, b)
    }
}

#[test]
fn test_functional() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());

    let a = Outer {
        inner: Renamed { a: 5, b: 7 },
        c: 1,
    };
    let b = Outer {
        inner: Renamed { a: 10, b: 14 },
        c: 2,
    };
    let c = ContractClient::new(&env, &contract_id).add(&a, &b);
    assert_eq!(c, (a, b));
}

#[test]
fn test_functional_with_original_type() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());

    let a = Outer {
        inner: inner::Inner { a: 5, b: 7 },
        c: 1,
    };
    let b = Outer {
        inner: inner::Inner { a: 10, b: 14 },
        c: 2,
    };
    let c = ContractClient::new(&env, &contract_id).add(&a, &b);
    assert_eq!(c, (a, b));
}

#[test]
fn test_spec() {
    let entries = ScSpecEntry::from_xdr(Outer::spec_xdr(), Limits::none()).unwrap();
    let expect = ScSpecEntry::UdtStructV0(ScSpecUdtStructV0 {
        doc: "".try_into().unwrap(),
        lib: "".try_into().unwrap(),
        name: "soroban_sdk::tests::contract_udt_struct_aliased_import::Outer"
            .try_into()
            .unwrap(),
        fields: vec![
            ScSpecUdtStructFieldV0 {
                doc: "".try_into().unwrap(),
                name: "c".try_into().unwrap(),
                type_: ScSpecTypeDef::I32,
            },
            ScSpecUdtStructFieldV0 {
                doc: "".try_into().unwrap(),
                name: "inner".try_into().unwrap(),
                type_: ScSpecTypeDef::Udt(ScSpecTypeUdt {
                    // See module doc comment: resolved through the type
                    // system to the referenced type's fully qualified
                    // definition path — the alias `Renamed` never appears.
                    name: "soroban_sdk::tests::contract_udt_struct_aliased_import::inner::Inner"
                        .try_into()
                        .unwrap(),
                }),
            },
        ]
        .try_into()
        .unwrap(),
    });
    assert_eq!(entries, expect);

    // Renamed's own spec entry is generated under the same fully qualified
    // definition path the field reference above resolves to, so the
    // reference and the entry match exactly.
    let entries = ScSpecEntry::from_xdr(Renamed::spec_xdr(), Limits::none()).unwrap();
    let expect = ScSpecEntry::UdtStructV0(ScSpecUdtStructV0 {
        doc: "".try_into().unwrap(),
        lib: "".try_into().unwrap(),
        name: "soroban_sdk::tests::contract_udt_struct_aliased_import::inner::Inner"
            .try_into()
            .unwrap(),
        fields: vec![
            ScSpecUdtStructFieldV0 {
                doc: "".try_into().unwrap(),
                name: "a".try_into().unwrap(),
                type_: ScSpecTypeDef::I32,
            },
            ScSpecUdtStructFieldV0 {
                doc: "".try_into().unwrap(),
                name: "b".try_into().unwrap(),
                type_: ScSpecTypeDef::I32,
            },
        ]
        .try_into()
        .unwrap(),
    });
    assert_eq!(entries, expect);
}
