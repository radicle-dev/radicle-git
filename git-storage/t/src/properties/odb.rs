// Copyright © 2022 The Radicle Link Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use git2::ObjectType;

use git_ref_format::RefString;
use git_ref_format_test::gen::valid;
use git_storage::{
    odb::{Read as _, Write as _},
    signature::UserInfo,
};

use proptest::prelude::*;

use crate::{gen, tmp};

// NOTE: It's enough to check the `Oid`s. If the contents were different the
// hash would be different.
pub mod prop {
    use super::*;

    pub fn roundtrip_blob(user: UserInfo, bytes: &[u8]) {
        let writer = tmp::writer(user);

        let oid = writer.write_blob(bytes).unwrap();

        let readback = writer.find_blob(oid).unwrap().unwrap();
        assert_eq!(oid, readback.id().into());

        let readback = writer.find_object(oid).unwrap().unwrap();
        assert_eq!(readback.kind(), Some(ObjectType::Blob));
    }

    pub fn roundtrip_tree(user: UserInfo, tree: gen::Tree) {
        let writer = tmp::writer(user);
        let oid = tree.write(&writer).unwrap();
        let readback = writer.find_tree(oid).unwrap().unwrap();
        assert_eq!(oid, readback.id().into());

        let readback = writer.find_object(oid).unwrap().unwrap();
        assert_eq!(readback.kind(), Some(ObjectType::Tree));
    }

    pub fn roundtrip_commit(user: UserInfo, tree: gen::Tree, message: &str) {
        let writer = tmp::writer(user);
        let tree_oid = tree.write(&writer).unwrap();
        let tree = writer.find_tree(tree_oid).unwrap().unwrap();
        let commit_oid = writer.write_commit(&tree, &[], message).unwrap();

        let readback = writer.find_commit(commit_oid).unwrap().unwrap();
        assert_eq!(readback.id(), commit_oid.into());

        let readback = writer.find_object(commit_oid).unwrap().unwrap();
        assert_eq!(readback.kind(), Some(ObjectType::Commit));
    }

    pub fn roundtrip_tag(user: UserInfo, tree: gen::Tree, tag_name: RefString, message: &str) {
        let writer = tmp::writer(user);
        let tree_oid = tree.write(&writer).unwrap();
        let tree = writer.find_tree(tree_oid).unwrap().unwrap();
        let commit_oid = writer.write_commit(&tree, &[], message).unwrap();
        let commit_object = writer.find_object(commit_oid).unwrap().unwrap();

        let tag_oid = writer.write_tag(tag_name, &commit_object, message).unwrap();

        let readback = writer.find_tag(tag_oid).unwrap().unwrap();
        assert_eq!(tag_oid, readback.id().into());

        let readback = writer.find_object(tag_oid).unwrap().unwrap();
        assert_eq!(readback.kind(), Some(ObjectType::Tag));
    }
}

proptest! {
    #[test]
    fn roundtrip_blob(user in gen::gen_signature(), bytes in gen::gen_bytes()) {
        prop::roundtrip_blob(user, &bytes)
    }


    #[test]
    fn roundtrip_tree(
        user in gen::gen_signature(),
        tree in gen::gen_tree(10),
    ) {
        prop::roundtrip_tree(user, tree)
    }

    #[test]
    fn roundtrip_commit(
        user in gen::gen_signature(),
        tree in gen::gen_tree(10),
        message in gen::trivial()
    ) {
        prop::roundtrip_commit(user, tree, &message)
    }

    #[test]
    fn roundtrip_tag(
        user in gen::gen_signature(),
        tree in gen::gen_tree(10),
        message in gen::trivial(),
        tag_name in valid()
    ) {
        let tag_name = RefString::try_from(tag_name).unwrap();
        prop::roundtrip_tag(user, tree, tag_name, &message);
    }
}
