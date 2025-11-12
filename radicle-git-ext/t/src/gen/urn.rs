use proptest::prelude::*;
use radicle_git_ext::Oid;

/// A proptest strategy that generates [`radicle_git_ext::oid::Oid`] values
/// of the type indicated in parameter `kind`
pub fn gen_oid(kind: git2::ObjectType) -> impl Strategy<Value = Oid> {
    any::<Vec<u8>>()
        .prop_map(move |bytes| git2::Oid::hash_object(kind, &bytes).map(Oid::from).unwrap())
}

/// A proptest strategy that generates [`radicle_git_ext::oid::Oid`] values
/// of the type indicated in parameter `kind` or a zeroed `Oid` with
/// equal probability
pub fn gen_oid_with_zero(kind: git2::ObjectType) -> impl Strategy<Value = Oid> {
    prop_oneof![gen_oid(kind), Just(git2::Oid::zero().into()),]
}
