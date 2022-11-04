# radicle-surf

## Overview

The main goal for the `radicle-surf` is to provide an API for
accessing a `git` repository and providing a code browsing
experience. This experience can be likened to GitHub or GitLab's
project browsing pages. It does not aim to be an UI layer, but rather
provides the functionality for a UI layer to built on top of it. With
that in mind, this document sets out to define the main components of
`radicle-surf` and a high-level design of the API.

Note that this is the second iteration of designing this library --
where the first can be found in the
[denotational-design.md][denotational] document. Since part of this
work is refactoring the original design there will be mention of
previous artefacts that are being changed. The original source code
can be found [here][radicle-surf], if you wish to study some history.

## Motivation

The `radicle-surf` crate aims to provide a safe and easy-to-use API that
supports the following features:

1. Code browsing: given a specific revision, browse files and directories.
2. Getting the difference between two revisions.
3. Retrieve the history of the commits, files, and directories.
4. Retrieve the references stored in the `git` project, e.g. branches,
   remote branches, tags, etc.
5. Retrieve specific `git` objects, in a user-friendly structure.

The main goals of the refactoring are:

* Reviewing the previous API and making it simpler where possible.
* Address open issues in the original [`radicle-surf`] project as much
  as possible.
* In contrast to the previous implementation, be `git` specific and
  not support other VCS systems.
* Hide away `git2` in the exposed API. The use of `git2` should be an
  implementation detail.

## API Review

Before defining the future design of the API, this document intends to
review the previous API to provide guidelines for building the future
version of the API.

### Remove `Browser`

The `Browser` started out succinct but became a kitchen sink for
functionality. Some if its problems include:

* It is not a source of truth of any information. For example,
  `list_branches` method is just a wrapper of
  `Repository::list_branches`.
* It takes in `History`, but really works at the `Snapshot` level.
* It is mutable but the state it holds is minimal and does not provide
  any use, beyond switching the `History`.

Going forward the removal of `Browser` is recommended. Some ways the
API will change with this removal are:

* For iteratoring the history, use `History`.
* For generating `Directory`, use the repository storage directly
  given a revision.
* For accessing references and objects use the repository storage
  directly.

### Remove `Snapshot`

A `Snapshot` was previously a function that converted a `History` into
a `Directory`. Since we can assume we are working in `git` this can be
simplified to a single function that can take a revision, that
resolves to a `Commit`, and produces a `Directory`.

## Components

The `radicle-surf` library can split into a few main components for
browsing a `git` repository, each of which will be discussed in the
following subsections.

Note that any of the API functions defined are _sketches_ and may not
be the final form or signature of the functions themselves. The traits
defined are recommendations, but other solutions for these
representations may be discovered during implementation of this design.

### References

Many are familiar with `git` branches. They are the main point of
interaction when working within a `git` repository. However, the more
general concept of a branch is a
[reference][git-references]. References are stored within the `git`
repository under the `refs/` directory. Within this directory, `git`
designates a few specific [namespaces][git-references]:

* `refs/heads` -- local branches
* `refs/remotes` -- remote branches
* `refs/tags` -- tagged `git` objects
* `refs/notes` -- attached notes to `git` references

These namespaces are designated special within `git`'s tooling, such
as the command line, however, there is nothing stopping one from
defining their own namespace, e.g. `refs/rad`.

As well as this, there is another way of separating `git` references
by a namespace which is achieved via the [gitnamespaces] feature. When
`GIT_NAMESPACE` or `--git-namespace` is set, the references are scoped
by `refs/namespaces/<namespace>`, e.g. if `GIT_NAMESPACE=rad` set then the
`refs/heads/main` branch would mean
`refs/namespaces/rad/refs/heads/main`.

With the above in mind, the following API functions are suggested:

```rust
/// Return a list of references based on the `pattern` string supplied, e.g. `refs/rad/*`
pub fn references(storage: &Storage, pattern: PatterStr) -> Result<References, Error>;

/// Return a list of branches based on the `pattern` string supplied, e.g. `refs/heads/features/*`
pub fn branches(storage: &Storage, pattern: BranchPattern) -> Result<Branches, Error>;

/// Return a list of remote branches based on the `pattern` string supplied, e.g. `refs/remotes/origin/features/*`
pub fn remotes(storage: &Storage, pattern: RemotePattern) -> Result<Remotes, Error>;

/// Return a list of tags based on the `pattern` string supplied, e.g. `refs/tags/releases/*`
pub fn tags(storage: &Storage, pattern: TagPattern) -> Result<Tags, Error>;

/// Return a list of notes based on the `pattern` string supplied, e.g. `refs/notes/blogs/*`
pub fn notes(storage: &Storage) -> Result<Notes, Error>;
```

It may be considered to be able to set an optional `gitnamespace`
within the storage, or ammend the pattern types to allow for scoping
by the `gitnamespace`.

The returned list will not be the objects themselves. Instead they
will be the metadata for those objects, i.e. `Oid`, `name`, etc. For
the retrieval of those objects see the section on
[Objects][#Objects]. The reason for this choice is that an UI may want
to avoid retrieving the actual object to limit the amount of data
needed. The `Oid` is the minimal amount of information required to
fetch the object itself.

### Revisions

Before describing the next components, it is important to first
describe [revisions][git-revisions]. A revision in `git` is a way of
specifying an `Oid`. This can be done in a multitude of ways. One can
also specify a range of `Oid`s (think `git log`). The API will support
taking revisions as parameters where an `Oid` is expected. It will
not, however, permit ranges (at least for the time being) and so a
revision will be scoped to any string that can resolve to a single
`Oid`, e.g. an `Oid` string itself, a reference name, `@{date}`, etc.
The aim will be to have a trait similar to:

```rust
/// `Self` is expected to be a type that can resolve to a single
/// `Oid`.
///
/// An `Oid` is the trivial case and returns itself, and is
/// infallible.
///
/// However, some other revisions require parsing and/or looking at the
/// storage, which may result in an `Error`.
pub trait FromRevision {
  type Error;

  /// Resolve the revision to its `Oid`, if possible.
  fn peel(&self, storage: &Storage) -> Result<Oid, Self::Error>;
}
```

### Objects

Within the `git` model, [references][#References] point to
[objects][git-objects]. The types of objects in `git` are: commits, tags (lightweight
& annotated), notes, trees, and blobs.

All of these objects can retrieved via their `Oid`. The API will
supply functions to retrieve them all for completion's sake, however,
we expect that retrieving commits, tags, and blobs will be the most
common usage.

```rust
/// Get the commit found by `oid`.
pub fn commit<R: FromRevision>(storage: &Storage, rev: R) -> Result<Commit, Error>;

/// Get the tag found by `oid`.
pub fn tag<R: FromRevision>(storage: &Storage, rev: R) -> Result<Tag, Error>;

/// Get the blob found by `oid`.
pub fn blob<R: FromRevision>(storage: &Storage, rev: R) -> Result<Blob, Error>;

/// Get the tree found by `oid`.
pub fn tree<R: FromRevision>(storage: &Storage, rev: R) -> Result<Tree, Error>;

/// Get the note found by `oid`.
pub fn note<R: FromRevision>(storage: &Storage, rev: R) -> Result<Note, Error>;
```

### Project Browsing

Project browsing boils down to taking a snapshot of a `git` repository
at a point in time and providing an object at that point in
time. Generally, this object would be a `Tree`, i.e. a directory of
files. However, it may be that a particular file, i.e. `Blob`, can be
viewed.

#### Commit-ish

The snapshot mentioned above is a `Commit` in `git`, where the
commit points to a `Tree` object. Thus, the API should be able to take
any parameter that may resolve to a `Commit`. This idea can be
captured as a trait, similar to `FromRevision`, which allows something
to be peeled to a `Commit`.

```rust
/// `Self` is expected to be a type that can resolve to a single
/// `Commit`.
///
/// A `Commit` is the trivial case and returns itself, and is
/// infallible.
///
/// However, some other kinds of data require parsing and/or looking at the
/// storage, which may result in an `Error`.
///
/// Common cases are:
///
///   * Reference that points to a commit `Oid`.
///   * A `Tag` that has a `target` of `Commit`.
///   * An `Oid` that is the identifier for a particular `Commit`.
pub trait Commitish {
  type Error;

  /// Resolve the type to its `Commit`, if possible.
  fn peel(&self, storage: &Storage) -> Result<Commit, Self::Error>;
}
```

This provides the building blocks for defining common cases of viewing
files and directories given a `Commitish` type.

```rust
/// Get the `Directory` found at `commit`.
pub fn directory<C: Commitish, P: AsRef<Path>>(
  storage: &Storage,
  commit: C,
  ) -> Result<Directory, Error>

/// Get the `File` found at `commit` under the given `path`.
pub fn file<C: Commitish, P: AsRef<Path>>(
  storage: &Storage,
  commit: C,
  path: P,
  ) -> Result<Option<File>, Error>
```

The `Directory` and `File` types above are deliberately opaque as
how they are defined falls out of scope of this document and should be
defined in an implementation specific design document.

### History

Since `Commit`s in `git` form a history of changes via a linked-list,
i.e. commits may have parents, and those parents grand-parents etc.,
it is important that an API for iterating through the history is
provided.

The general mechanism for looking at a history of commits is called a
[`revwalk`][libgit-revwal] in most `git` libraries. This provides a
lazy iterator over the history, which will be useful for limiting the
memory consumption of any implementation.

```rust
// history.rs

/// Return an iterator of the `Directory`'s history, beginning with
/// the `start` provided.
pub fn directory<C: Commitish>(start: C) -> History<Directory>

/// Return an iterator of the `file`'s history, beginning with the
/// `start` provided.
pub fn file<C: Commitish>(start: C, file: File) -> History<File>

/// Return an iterator of the `Commit`'s history, beginning with the
/// `start` provided.
pub fn commit<C: Commitish>(start: C) -> History<Commit>
```

The `History` type above are deliberately opaque as how it is defined
falls out of scope of this document and should be defined in an
implementation specific design document.

### Diffs

The final component for a good project browsing experience is being
able to look at the difference between two snapshots in time. This is
colloquially shortened to the term "diff" (or diffs for plural).

Since diffs are between two snapshots, the expected API should take
two `Commit`s that resolve to `Tree`s.

```rust
/// New type to differentiate the old side of a [`diff`].
pub struct Old<C>(C);

/// New type to differentiate the new side of a [`diff`].
pub struct New<C>(C);

/// Get the difference between the `old` and the `new` directories.
pub fn diff<C: Commitish>(old: Old<C>, new: New<C>) -> Result<Diff, Error>
```

## Conclusion

This document has provided the foundations for building the
`radicle-surf` API. It has provided a sketch of the functionality of
each of the subcomponents -- which should naturally feed into each
other -- and some recommended traits for making the API easier to
use. A futher document should be specified for a specific Rust
implementation.

[denotational]: https://github.com/radicle-dev/radicle-git/blob/main/radicle-surf/docs/denotational-design.md
[gitnamespaces]: https://git-scm.com/docs/gitnamespaces
[git-objects]: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects
[git-references]: https://git-scm.com/book/en/v2/Git-Internals-Git-References
[git-revisions]: https://git-scm.com/docs/revision
[libgit-revwalk]: https://github.com/libgit2/libgit2/blob/main/include/git2/revwalk.h
[radicle-surf]: https://github.com/radicle-dev/radicle-surf
