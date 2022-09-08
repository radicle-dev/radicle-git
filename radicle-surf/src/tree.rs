// This file is part of radicle-surf
// <https://github.com/radicle-dev/radicle-surf>
//
// Copyright (C) 2019-2020 The Radicle Team <dev@radicle.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 or
// later as published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::nonempty::split_last;
use nonempty::NonEmpty;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubTree<K, A> {
    Node { key: K, value: A },
    Branch { key: K, forest: Box<Tree<K, A>> },
}

impl<K, A> SubTree<K, A> {
    /// Create a new `Branch` from a key and sub-tree.
    ///
    /// This function is a convenience for now having to
    /// remember to use `Box::new`.
    fn branch(key: K, tree: Tree<K, A>) -> Self {
        SubTree::Branch {
            key,
            forest: Box::new(tree),
        }
    }

    fn key(&self) -> &K {
        match self {
            SubTree::Node { key, .. } => key,
            SubTree::Branch { key, .. } => key,
        }
    }

    pub fn find(&self, keys: NonEmpty<K>) -> Option<&Self>
    where
        K: Ord,
    {
        let (head, tail) = keys.into();
        let tail = NonEmpty::from_vec(tail);
        match self {
            SubTree::Node { key, .. } => match tail {
                None if *key == head => Some(self),
                _ => None,
            },
            SubTree::Branch { key, ref forest } => match tail {
                None if *key == head => Some(self),
                None => None,
                Some(keys) => forest.find(keys),
            },
        }
    }

    pub fn to_nonempty(&self) -> NonEmpty<A>
    where
        A: Clone,
        K: Clone,
    {
        match self {
            Self::Node { value, .. } => NonEmpty::new(value.clone()),
            Self::Branch { forest, .. } => forest.to_nonempty(),
        }
    }

    pub(crate) fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &A> + 'a> {
        match self {
            SubTree::Node { value, .. } => Box::new(std::iter::once(value)),
            SubTree::Branch { ref forest, .. } => Box::new(forest.iter()),
        }
    }

    fn iter_keys<'a>(&'a self) -> Box<dyn Iterator<Item = &K> + 'a> {
        match self {
            SubTree::Node { key, .. } => Box::new(std::iter::once(key)),
            SubTree::Branch {
                ref key,
                ref forest,
            } => Box::new(std::iter::once(key).chain(forest.iter_keys())),
        }
    }

    fn compare_by<F>(&self, other: &Self, f: &F) -> Ordering
    where
        F: Fn(&A, &A) -> Ordering,
    {
        match (self, other) {
            (
                SubTree::Node { value, .. },
                SubTree::Node {
                    value: other_value, ..
                },
            ) => f(value, other_value),
            (SubTree::Branch { forest, .. }, SubTree::Node { value, .. }) => {
                let max_forest = forest.maximum_by(f);
                f(max_forest, value)
            },
            (SubTree::Node { value, .. }, SubTree::Branch { forest, .. }) => {
                let max_forest = &forest.maximum_by(f);
                f(value, max_forest)
            },
            (
                SubTree::Branch { forest, .. },
                SubTree::Branch {
                    forest: other_forest,
                    ..
                },
            ) => {
                let max_forest = forest.maximum_by(f);
                let max_other_forest = other_forest.maximum_by(f);
                f(max_forest, max_other_forest)
            },
        }
    }

    pub fn maximum_by<F>(&self, f: &F) -> &A
    where
        F: Fn(&A, &A) -> Ordering,
    {
        match self {
            SubTree::Node { value, .. } => value,
            SubTree::Branch { forest, .. } => forest.maximum_by(f),
        }
    }

    pub fn map<F, B>(self, f: &mut F) -> SubTree<K, B>
    where
        F: FnMut(A) -> B,
    {
        match self {
            SubTree::Node { key, value } => SubTree::Node {
                key,
                value: f(value),
            },
            SubTree::Branch { key, forest } => SubTree::Branch {
                key,
                forest: Box::new(forest.map(f)),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tree<K, A>(pub(crate) NonEmpty<SubTree<K, A>>);

impl<K, A> From<Tree<K, A>> for Forest<K, A> {
    fn from(tree: Tree<K, A>) -> Self {
        Forest(Some(tree))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Forest<K, A>(pub(crate) Option<Tree<K, A>>);

impl<K, A> Tree<K, A> {
    /// Create a new `Tree` containing a single `Branch` given
    /// the key and sub-tree.
    fn branch(key: K, forest: Self) -> Self {
        Tree(NonEmpty::new(SubTree::branch(key, forest)))
    }

    /// Create a new `Tree` containing a single `Node`.
    fn node(key: K, value: A) -> Self {
        Tree(NonEmpty::new(SubTree::Node { key, value }))
    }

    /// Create a new `Tree` that creates a series of
    /// `Branch`es built using the `keys`. The final `Branch`
    /// will contain the `node`.
    fn new(keys: NonEmpty<K>, node: A) -> Self
    where
        K: Ord,
    {
        let (start, mut middle) = keys.into();
        let last = middle.pop();

        match last {
            None => Tree::node(start, node),
            Some(last) => {
                let mut branch = Tree::node(last, node);

                for key in middle.into_iter().rev() {
                    branch = Tree(NonEmpty::new(SubTree::branch(key, branch)))
                }

                Tree::branch(start, branch)
            },
        }
    }

    /// Perform a binary search in the sub-trees, based on comparing
    /// each of the sub-trees' key to the provided `key`.
    fn search(&self, key: &K) -> Result<usize, usize>
    where
        K: Ord,
    {
        self.0.binary_search_by(|tree| tree.key().cmp(key))
    }

    pub fn map<F, B>(self, mut f: F) -> Tree<K, B>
    where
        F: FnMut(A) -> B,
    {
        Tree(self.0.map(|tree| tree.map(&mut f)))
    }

    /// Insert a `node` into the list of sub-trees.
    ///
    /// The node's position will be based on the `Ord` instance
    /// of `K`.
    fn insert_node_with<F>(&mut self, key: K, value: A, f: F)
    where
        F: FnOnce(&mut A),
        K: Ord,
    {
        let result = self.search(&key);

        match result {
            Ok(index) => {
                let old_node = self.0.get_mut(index).unwrap();
                match old_node {
                    SubTree::Node { value: old, .. } => f(old),
                    SubTree::Branch { .. } => *old_node = SubTree::Node { key, value },
                }
            },
            Err(index) => self.0.insert(index, SubTree::Node { key, value }),
        }
    }

    /// Insert the `node` in the position given by `keys`.
    ///
    /// If the same path to a node is provided the `node` will replace the old
    /// one, i.e. if `a/b/c` exists in the tree and `a/b/c` is the full path
    /// to the node, then `c` will be replaced.
    ///
    /// If the path points to a branch, then the `node` will be inserted in this
    /// branch.
    ///
    /// If a portion of the path points to a node then a branch will be created
    /// in its place, i.e. if `a/b/c` exists in the tree and the provided
    /// path is `a/b/c/d`, then the node `c` will be replaced by a branch
    /// `c/d`.
    ///
    /// If the path does not exist it will be inserted into the set of
    /// sub-trees.
    fn insert_with<F>(&mut self, keys: NonEmpty<K>, value: A, f: F)
    where
        F: FnOnce(&mut A),
        K: Ord,
    {
        let (head, tail) = keys.into();
        let maybe_keys = NonEmpty::from_vec(tail);
        match self.search(&head) {
            // Found the label in our set of sub-trees
            Ok(index) => match maybe_keys {
                // The keys have been exhausted and so its time to insert the node
                None => {
                    let sub_tree = self.0.get_mut(index).unwrap();
                    match sub_tree {
                        // Our sub-tree was a node.
                        SubTree::Node { key, value } => {
                            let _ = std::mem::replace(key, head);
                            f(value);
                        },
                        SubTree::Branch { .. } => *sub_tree = SubTree::Node { key: head, value },
                    }
                },
                Some(keys) => {
                    let sub_tree = self.0.get_mut(index).unwrap();
                    match sub_tree {
                        // We have reached a node, but still have keys left to get through.
                        SubTree::Node { .. } => {
                            let new_tree = SubTree::branch(head, Tree::new(keys, value));
                            *sub_tree = new_tree
                        },
                        // We keep moving down the set of keys to find where to insert this node.
                        SubTree::Branch { forest, .. } => forest.insert_with(keys, value, f),
                    }
                },
            },
            // The label was not found and we have an index for insertion.
            Err(index) => match maybe_keys {
                // We create the branch with the head label and node, since there are
                // no more labels left.
                None => self.0.insert(index, SubTree::Node { key: head, value }),
                // We insert an entirely new branch with the full list of keys.
                Some(tail) => self
                    .0
                    .insert(index, SubTree::branch(head, Tree::new(tail, value))),
            },
        }
    }

    pub fn insert(&mut self, keys: NonEmpty<K>, value: A)
    where
        A: Clone,
        K: Ord,
    {
        self.insert_with(keys, value.clone(), |old| *old = value)
    }

    pub fn to_nonempty(&self) -> NonEmpty<A>
    where
        A: Clone,
        K: Clone,
    {
        self.0.clone().flat_map(|sub_tree| sub_tree.to_nonempty())
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &A> + 'a {
        self.0.iter().flat_map(|tree| tree.iter())
    }

    pub fn iter_keys<'a>(&'a self) -> impl Iterator<Item = &K> + 'a {
        self.0.iter().flat_map(|tree| tree.iter_keys())
    }

    pub fn iter_subtrees<'a>(&'a self) -> impl Iterator<Item = &SubTree<K, A>> + 'a {
        self.0.iter()
    }

    pub fn find_node(&self, keys: NonEmpty<K>) -> Option<&A>
    where
        K: Ord,
    {
        self.find(keys).and_then(|tree| match tree {
            SubTree::Node { value, .. } => Some(value),
            SubTree::Branch { .. } => None,
        })
    }

    pub fn find_branch(&self, keys: NonEmpty<K>) -> Option<&Self>
    where
        K: Ord,
    {
        self.find(keys).and_then(|tree| match tree {
            SubTree::Node { .. } => None,
            SubTree::Branch { ref forest, .. } => Some(&**forest),
        })
    }

    /// Find a `SubTree` given a search path. If the path does not match
    /// it will return `None`.
    pub fn find(&self, keys: NonEmpty<K>) -> Option<&SubTree<K, A>>
    where
        K: Ord,
    {
        let (head, tail) = keys.into();
        let tail = NonEmpty::from_vec(tail);
        match self.search(&head) {
            Err(_) => None,
            Ok(index) => {
                let sub_tree = self.0.get(index).unwrap();
                match tail {
                    None => match sub_tree {
                        SubTree::Node { .. } => Some(sub_tree),
                        SubTree::Branch { .. } => Some(sub_tree),
                    },
                    Some(mut tail) => {
                        tail.insert(0, head);
                        sub_tree.find(tail)
                    },
                }
            },
        }
    }

    pub fn maximum_by<F>(&self, f: &F) -> &A
    where
        F: Fn(&A, &A) -> Ordering,
    {
        self.0.maximum_by(|s, t| s.compare_by(t, f)).maximum_by(f)
    }

    #[allow(dead_code)]
    pub fn maximum(&self) -> &A
    where
        A: Ord,
    {
        self.maximum_by(&|a, b| a.cmp(b))
    }
}

impl<K, A> Forest<K, A> {
    pub fn root() -> Self {
        Forest(None)
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    fn insert_forest(&mut self, forest: Tree<K, A>) {
        self.0 = Some(forest)
    }

    /// Insert the `node` in the position given by `keys`.
    ///
    /// If the same path to a node is provided the `node` will replace the old
    /// one, i.e. if `a/b/c` exists in the tree and `a/b/c` is the full path
    /// to the node, then `c` will be replaced.
    ///
    /// If the path points to a branch, then the `node` will be inserted in this
    /// branch.
    ///
    /// If a portion of the path points to a node then a branch will be created
    /// in its place, i.e. if `a/b/c` exists in the tree and the provided
    /// path is `a/b/c/d`, then the node `c` will be replaced by a branch
    /// `c/d`.
    ///
    /// If the path does not exist it will be inserted into the set of
    /// sub-trees.
    #[allow(dead_code)]
    pub fn insert(&mut self, keys: NonEmpty<K>, node: A)
    where
        A: Clone,
        K: Ord,
    {
        self.insert_with(keys, node.clone(), |old| *old = node)
    }

    pub fn insert_with<F>(&mut self, keys: NonEmpty<K>, node: A, f: F)
    where
        F: FnOnce(&mut A),
        K: Ord,
    {
        let (prefix, node_key) = split_last(keys);
        match self.0.as_mut() {
            Some(forest) => match NonEmpty::from_vec(prefix) {
                None => {
                    // Insert the node at the root
                    forest.insert_node_with(node_key, node, f)
                },
                Some(mut keys) => {
                    keys.push(node_key);
                    forest.insert_with(keys, node, f)
                },
            },
            None => match NonEmpty::from_vec(prefix) {
                None => self.insert_forest(Tree::node(node_key, node)),
                Some(mut keys) => {
                    keys.push(node_key);
                    self.insert_forest(Tree::new(keys, node))
                },
            },
        }
    }

    pub fn find_node(&self, keys: NonEmpty<K>) -> Option<&A>
    where
        K: Ord,
    {
        self.0.as_ref().and_then(|trees| trees.find_node(keys))
    }

    pub fn find_branch(&self, keys: NonEmpty<K>) -> Option<&Tree<K, A>>
    where
        K: Ord,
    {
        self.0.as_ref().and_then(|trees| trees.find_branch(keys))
    }

    #[allow(dead_code)]
    /// Find a `SubTree` given a search path. If the path does not match
    /// it will return `None`.
    pub fn find(&self, keys: NonEmpty<K>) -> Option<&SubTree<K, A>>
    where
        K: Ord,
    {
        self.0.as_ref().and_then(|trees| trees.find(keys))
    }

    #[allow(dead_code)]
    pub fn maximum_by<F>(&self, f: F) -> Option<&A>
    where
        F: Fn(&A, &A) -> Ordering,
    {
        self.0.as_ref().map(|trees| trees.maximum_by(&f))
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &A> + 'a {
        self.0.iter().flat_map(|trees| trees.iter())
    }

    #[allow(dead_code)]
    pub fn iter_keys<'a>(&'a self) -> impl Iterator<Item = &K> + 'a {
        self.0.iter().flat_map(|trees| trees.iter_keys())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

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
}
