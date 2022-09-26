// Copyright © 2022 The Radicle Git Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Unit tests for radicle_surf::tree.

use nonempty::NonEmpty;
use pretty_assertions::assert_eq;
use radicle_surf::tree::{Forest, SubTree, Tree};

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestNode {
    id: u32,
}

#[test]
fn test_is_empty() {
    let mut tree = Forest::root();
    assert!(tree.is_empty());

    let a_node = TestNode { id: 1 };

    tree.insert(NonEmpty::new(String::from("a")), a_node);
    assert!(!tree.is_empty());
}

#[test]
fn test_insert_root_node() {
    let a_label = String::from("a");

    let mut tree = Forest::root();

    let a_node = TestNode { id: 1 };

    tree.insert(NonEmpty::new(a_label), a_node.clone());

    assert_eq!(tree, Forest(Some(Tree::node(String::from("a"), a_node))));
}

#[test]
fn test_insert_with_prepending_root_nodes() {
    let a_label = String::from("a");

    let mut tree = Forest::root();

    let a_node = TestNode { id: 1 };
    let b_node = TestNode { id: 2 };

    tree.insert_with(
        NonEmpty::new(a_label.clone()),
        NonEmpty::new(a_node.clone()),
        |nodes| nodes.insert(0, a_node.clone()),
    );
    tree.insert_with(
        NonEmpty::new(a_label),
        NonEmpty::new(b_node.clone()),
        |nodes| nodes.insert(0, b_node.clone()),
    );

    assert_eq!(
        tree,
        Forest(Some(Tree::node(
            String::from("a"),
            NonEmpty::from((b_node, vec![a_node]))
        )))
    );
}

#[test]
fn test_insert_with_prepending_branch_nodes() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let path = NonEmpty::from((a_label, vec![b_label]));

    let mut tree = Forest::root();

    let a_node = TestNode { id: 1 };
    let b_node = TestNode { id: 2 };

    tree.insert_with(path.clone(), NonEmpty::new(a_node.clone()), |nodes| {
        nodes.insert(0, a_node.clone())
    });
    tree.insert_with(path, NonEmpty::new(b_node.clone()), |nodes| {
        nodes.insert(0, b_node.clone())
    });

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree::node(String::from("b"), NonEmpty::from((b_node, vec![a_node])))
        )))
    );
}

#[test]
fn test_insert_single_node() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let path = NonEmpty::from((a_label, vec![b_label, c_label]));

    let mut tree = Forest::root();

    let c_node = TestNode { id: 1 };

    tree.insert(path, c_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree::branch(String::from("b"), Tree::node(String::from("c"), c_node))
        )))
    );
}

#[test]
fn test_insert_two_nodes() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let d_label = String::from("d");
    let c_path = NonEmpty::from((a_label.clone(), vec![b_label.clone(), c_label]));
    let d_path = NonEmpty::from((a_label, vec![b_label, d_label]));

    let mut tree = Forest::root();

    let c_node = TestNode { id: 1 };

    tree.insert(c_path, c_node.clone());

    let d_node = TestNode { id: 3 };

    tree.insert(d_path, d_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree::branch(
                String::from("b"),
                Tree(NonEmpty::from((
                    SubTree::Node {
                        key: String::from("c"),
                        value: c_node
                    },
                    vec![SubTree::Node {
                        key: String::from("d"),
                        value: d_node
                    }]
                )))
            )
        )))
    );
}

#[test]
fn test_insert_replaces_node() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let c_path = NonEmpty::from((a_label, vec![b_label, c_label]));

    let mut tree = Forest::root();

    let c_node = TestNode { id: 1 };

    tree.insert(c_path.clone(), c_node);

    let new_c_node = TestNode { id: 3 };

    tree.insert(c_path, new_c_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree::branch(
                String::from("b"),
                Tree(NonEmpty::new(SubTree::Node {
                    key: String::from("c"),
                    value: new_c_node
                },))
            )
        )))
    );
}

#[test]
fn test_insert_replaces_root_node() {
    let c_label = String::from("c");

    let mut tree = Forest::root();

    let c_node = TestNode { id: 1 };

    tree.insert(NonEmpty::new(c_label.clone()), c_node);

    let new_c_node = TestNode { id: 3 };

    tree.insert(NonEmpty::new(c_label), new_c_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::node(String::from("c"), new_c_node)))
    );
}

#[test]
fn test_insert_replaces_branch_node() {
    let a_label = String::from("a");
    let c_label = String::from("c");
    let c_path = NonEmpty::from((a_label, vec![c_label]));

    let mut tree = Forest::root();

    let c_node = TestNode { id: 1 };

    tree.insert(c_path.clone(), c_node);

    let new_c_node = TestNode { id: 3 };

    tree.insert(c_path, new_c_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree::node(String::from("c"), new_c_node),
        )))
    );
}

#[test]
fn test_insert_replaces_branch_with_node() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let c_path = NonEmpty::from((a_label.clone(), vec![b_label.clone(), c_label]));

    let mut tree = Forest::root();

    let c_node = TestNode { id: 1 };

    tree.insert(c_path, c_node);

    let new_c_node = TestNode { id: 3 };

    tree.insert(NonEmpty::from((a_label, vec![b_label])), new_c_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree::node(String::from("b"), new_c_node),
        )))
    );
}

#[test]
fn test_insert_replaces_node_with_branch() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let b_path = NonEmpty::from((a_label.clone(), vec![b_label.clone()]));
    let c_path = NonEmpty::from((a_label, vec![b_label, c_label]));

    let mut tree = Forest::root();

    let b_node = TestNode { id: 1 };

    tree.insert(b_path, b_node);

    let new_c_node = TestNode { id: 3 };

    tree.insert(c_path, new_c_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree::branch(
                String::from("b"),
                Tree(NonEmpty::new(SubTree::Node {
                    key: String::from("c"),
                    value: new_c_node
                },))
            )
        )))
    );
}

#[test]
fn test_insert_replaces_node_with_branch_foo() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let d_label = String::from("d");
    let b_path = NonEmpty::from((a_label.clone(), vec![b_label.clone()]));
    let d_path = NonEmpty::from((a_label, vec![b_label, c_label, d_label]));

    let mut tree = Forest::root();

    let b_node = TestNode { id: 1 };

    tree.insert(b_path, b_node);

    let d_node = TestNode { id: 3 };

    tree.insert(d_path, d_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree::branch(
                String::from("b"),
                Tree::branch(String::from("c"), Tree::node(String::from("d"), d_node))
            )
        )))
    );
}

#[test]
fn test_insert_two_nodes_out_of_order() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let d_label = String::from("d");
    let c_path = NonEmpty::from((a_label.clone(), vec![b_label.clone(), c_label]));
    let d_path = NonEmpty::from((a_label, vec![b_label, d_label]));

    let mut tree = Forest::root();

    let d_node = TestNode { id: 3 };

    tree.insert(d_path, d_node.clone());

    let c_node = TestNode { id: 1 };

    tree.insert(c_path, c_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree::branch(
                String::from("b"),
                Tree(NonEmpty::from((
                    SubTree::Node {
                        key: String::from("c"),
                        value: c_node
                    },
                    vec![SubTree::Node {
                        key: String::from("d"),
                        value: d_node
                    }]
                )))
            )
        )))
    );
}

#[test]
fn test_insert_branch() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let d_label = String::from("d");
    let e_label = String::from("e");
    let f_label = String::from("f");

    let c_path = NonEmpty::from((a_label.clone(), vec![b_label.clone(), c_label]));
    let d_path = NonEmpty::from((a_label.clone(), vec![b_label, d_label]));
    let f_path = NonEmpty::from((a_label, vec![e_label, f_label]));

    let mut tree = Forest::root();

    let c_node = TestNode { id: 1 };

    let d_node = TestNode { id: 3 };

    let f_node = TestNode { id: 2 };

    tree.insert(d_path, d_node.clone());
    tree.insert(c_path, c_node.clone());
    tree.insert(f_path, f_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree(NonEmpty::from((
                SubTree::Branch {
                    key: String::from("b"),
                    forest: Box::new(Tree(NonEmpty::from((
                        SubTree::Node {
                            key: String::from("c"),
                            value: c_node
                        },
                        vec![SubTree::Node {
                            key: String::from("d"),
                            value: d_node
                        }]
                    ))))
                },
                vec![SubTree::Branch {
                    key: String::from("e"),
                    forest: Box::new(Tree::node(String::from("f"), f_node))
                },]
            )))
        )))
    );
}

#[test]
fn test_insert_two_branches() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let d_label = String::from("d");
    let e_label = String::from("e");
    let f_label = String::from("f");

    let c_path = NonEmpty::from((a_label, vec![b_label, c_label]));
    let f_path = NonEmpty::from((d_label, vec![e_label, f_label]));

    let mut tree = Forest::root();

    let c_node = TestNode { id: 1 };

    let f_node = TestNode { id: 2 };

    tree.insert(c_path, c_node.clone());
    tree.insert(f_path, f_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree(NonEmpty::from((
            SubTree::Branch {
                key: String::from("a"),
                forest: Box::new(Tree::branch(
                    String::from("b"),
                    Tree::node(String::from("c"), c_node)
                )),
            },
            vec![SubTree::Branch {
                key: String::from("d"),
                forest: Box::new(Tree::branch(
                    String::from("e"),
                    Tree::node(String::from("f"), f_node)
                ))
            }]
        )))))
    );
}

#[test]
fn test_insert_branches_and_node() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let d_label = String::from("d");
    let e_label = String::from("e");
    let f_label = String::from("f");
    let g_label = String::from("g");

    let c_path = NonEmpty::from((a_label.clone(), vec![b_label.clone(), c_label]));
    let d_path = NonEmpty::from((a_label.clone(), vec![b_label, d_label]));
    let e_path = NonEmpty::from((a_label.clone(), vec![e_label]));
    let g_path = NonEmpty::from((a_label, vec![f_label, g_label]));

    let mut tree = Forest::root();

    let c_node = TestNode { id: 1 };

    let d_node = TestNode { id: 3 };

    let e_node = TestNode { id: 2 };

    let g_node = TestNode { id: 2 };

    tree.insert(d_path, d_node.clone());
    tree.insert(c_path, c_node.clone());
    tree.insert(e_path, e_node.clone());
    tree.insert(g_path, g_node.clone());

    assert_eq!(
        tree,
        Forest(Some(Tree::branch(
            String::from("a"),
            Tree(NonEmpty::from((
                SubTree::Branch {
                    key: String::from("b"),
                    forest: Box::new(Tree(NonEmpty::from((
                        SubTree::Node {
                            key: String::from("c"),
                            value: c_node
                        },
                        vec![SubTree::Node {
                            key: String::from("d"),
                            value: d_node
                        }]
                    ))))
                },
                vec![
                    SubTree::Node {
                        key: String::from("e"),
                        value: e_node
                    },
                    SubTree::Branch {
                        key: String::from("f"),
                        forest: Box::new(Tree::node(String::from("g"), g_node))
                    },
                ]
            )))
        )))
    );
}

#[test]
fn test_find_root_node() {
    let a_label = String::from("a");

    let mut tree = Forest::root();

    let a_node = TestNode { id: 1 };

    tree.insert(NonEmpty::new(a_label), a_node.clone());

    assert_eq!(
        tree.find(NonEmpty::new(String::from("a"))),
        Some(&SubTree::Node {
            key: String::from("a"),
            value: a_node
        })
    );

    assert_eq!(tree.find(NonEmpty::new(String::from("b"))), None);
}

#[test]
fn test_find_branch_and_node() {
    let a_label = String::from("a");
    let b_label = String::from("b");
    let c_label = String::from("c");
    let path = NonEmpty::from((a_label, vec![b_label, c_label]));

    let mut tree = Forest::root();

    let c_node = TestNode { id: 1 };

    tree.insert(path, c_node.clone());

    assert_eq!(
        tree.find(NonEmpty::new(String::from("a"))),
        Some(&SubTree::Branch {
            key: String::from("a"),
            forest: Box::new(Tree::branch(
                String::from("b"),
                Tree::node(String::from("c"), c_node.clone())
            ))
        })
    );

    assert_eq!(
        tree.find(NonEmpty::from((String::from("a"), vec![String::from("b")]))),
        Some(&SubTree::Branch {
            key: String::from("b"),
            forest: Box::new(Tree::node(String::from("c"), c_node.clone()))
        })
    );

    assert_eq!(
        tree.find(NonEmpty::from((
            String::from("a"),
            vec![String::from("b"), String::from("c")]
        ))),
        Some(&SubTree::Node {
            key: String::from("c"),
            value: c_node
        })
    );

    assert_eq!(tree.find(NonEmpty::new(String::from("b"))), None);

    assert_eq!(
        tree.find(NonEmpty::from((String::from("a"), vec![String::from("c")]))),
        None
    );
}

#[test]
fn test_maximum_by_root_nodes() {
    let mut tree = Forest::root();

    let a_node = TestNode { id: 1 };

    let b_node = TestNode { id: 3 };

    tree.insert(NonEmpty::new(String::from("a")), a_node.clone());
    tree.insert(NonEmpty::new(String::from("b")), b_node.clone());

    assert_eq!(tree.maximum_by(|a, b| a.id.cmp(&b.id)), Some(&b_node));
    assert_eq!(
        tree.maximum_by(|a, b| a.id.cmp(&b.id).reverse()),
        Some(&a_node)
    );
}

#[test]
fn test_maximum_by_branch_and_node() {
    let mut tree = Forest::root();

    let a_node = TestNode { id: 1 };

    let b_node = TestNode { id: 3 };

    tree.insert(
        NonEmpty::from((String::from("c"), vec![String::from("a")])),
        a_node.clone(),
    );
    tree.insert(NonEmpty::new(String::from("b")), b_node.clone());

    assert_eq!(tree.maximum_by(|a, b| a.id.cmp(&b.id)), Some(&b_node));
    assert_eq!(
        tree.maximum_by(|a, b| a.id.cmp(&b.id).reverse()),
        Some(&a_node)
    );
}

#[test]
fn test_maximum_by_branch_and_branch() {
    let mut tree = Forest::root();

    let a_node = TestNode { id: 1 };

    let b_node = TestNode { id: 3 };

    tree.insert(
        NonEmpty::from((String::from("c"), vec![String::from("a")])),
        a_node.clone(),
    );
    tree.insert(
        NonEmpty::from((String::from("d"), vec![String::from("a")])),
        b_node.clone(),
    );

    assert_eq!(tree.maximum_by(|a, b| a.id.cmp(&b.id)), Some(&b_node));
    assert_eq!(
        tree.maximum_by(|a, b| a.id.cmp(&b.id).reverse()),
        Some(&a_node)
    );
}

#[test]
fn test_maximum_by_branch_nodes() {
    let mut tree = Forest::root();

    let a_node = TestNode { id: 1 };

    let b_node = TestNode { id: 3 };

    tree.insert(
        NonEmpty::from((String::from("c"), vec![String::from("a")])),
        a_node.clone(),
    );
    tree.insert(
        NonEmpty::from((String::from("c"), vec![String::from("b")])),
        b_node.clone(),
    );

    assert_eq!(tree.maximum_by(|a, b| a.id.cmp(&b.id)), Some(&b_node));
    assert_eq!(
        tree.maximum_by(|a, b| a.id.cmp(&b.id).reverse()),
        Some(&a_node)
    );
}

#[test]
fn test_fold_root_nodes() {
    let mut tree = Forest::root();

    let a_node = TestNode { id: 1 };

    let b_node = TestNode { id: 3 };

    tree.insert(NonEmpty::new(String::from("a")), a_node);
    tree.insert(NonEmpty::new(String::from("b")), b_node);

    assert_eq!(tree.iter().fold(0, |b, a| a.id + b), 4);
}
