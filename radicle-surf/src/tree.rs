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

#![allow(missing_docs)]

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
pub struct Tree<K, A>(pub NonEmpty<SubTree<K, A>>);

impl<K, A> From<Tree<K, A>> for Forest<K, A> {
    fn from(tree: Tree<K, A>) -> Self {
        Forest(Some(tree))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Forest<K, A>(pub Option<Tree<K, A>>);

impl<K, A> Tree<K, A> {
    /// Create a new `Tree` containing a single `Branch` given
    /// the key and sub-tree.
    pub fn branch(key: K, forest: Self) -> Self {
        Tree(NonEmpty::new(SubTree::branch(key, forest)))
    }

    /// Create a new `Tree` containing a single `Node`.
    pub fn node(key: K, value: A) -> Self {
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

    pub fn iter(&self) -> impl Iterator<Item = &A> {
        self.0.iter().flat_map(|tree| tree.iter())
    }

    pub fn iter_keys(&self) -> impl Iterator<Item = &K> {
        self.0.iter().flat_map(|tree| tree.iter_keys())
    }

    pub fn iter_subtrees(&self) -> impl Iterator<Item = &SubTree<K, A>> {
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

    pub fn iter(&self) -> impl Iterator<Item = &A> {
        self.0.iter().flat_map(|trees| trees.iter())
    }

    #[allow(dead_code)]
    pub fn iter_keys(&self) -> impl Iterator<Item = &K> {
        self.0.iter().flat_map(|trees| trees.iter_keys())
    }
}
