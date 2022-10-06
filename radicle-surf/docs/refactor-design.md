# An updated design for radicle-surf

## Introduction

Now we have ported the `radicle-surf` crate from its own github repo to be
part of the `radicle-git` repo. We are taking this opportunity to refactor
its design as well. Intuitively, `radicle-surf` provides an API so that one
can use it to create a GitHub-like UI for a git repo:

1. Code browsing: given a specific commit/ref, browse files and directories.
2. Diff between two revisions that resolve into two commits.
3. Retrieve the history of the commits.
4. Retrieve a specific object: all its metadata.
5. Retrieve the refs: Branches, Tags, Remotes, Notes and user-defined
"categories", where a category is: refs/<category>/<...>.

## Motivation

The `radicle-surf` crate aims to provide a safe and easy-to-use API that
supports the features listed in [Introduction]. Based on the existing API,
the main goals of the refactoring are:

- API review: make API simpler whenever possible.
- Address open issues in the original `radicle-surf` repo as much as possible.
- Not to shy away from being `git` specific. (i.e. not to consider supporting
other VCS systems)
- Hide away `git2` from the API. The use of `git2` should be an implementation
detail.

## API review

The current API has quite a bit accidental complexity that is not inherent with
the requirements, especially when we can be git specific and don't care about
other VCS systems.

### Remove the `Browser`

The type `Browser` is awkward as of today:

- it is not a source of truth of any information. For example, `list_branches`
method is just a wrapper of `Repository::list_branches`.
- it takes in `History`, but really works at the `Snapshot` level.
- it is mutable but its state does not help much.

Can we just remove `Browser` and implement its functionalities using other
types?

- For iteratoring the history, use `History`.
- For generating `Directory`, use `Repository` directly given a `Rev`.
- For accessing `Branch`, `Tag` or `Commit`, use `Repository`.

## Remove the `Snapshot` type

A `Snapshot` should be really just a tree (or `Directory`) of a `Commit` in
git. Currently it is a function that returns a `Directory`. Because it is OK
to be git specific, we don't need to have this generic function to create a
snapshot across different VCS systems.

The snapshot function can be easily implement as a method of `RepositoryRef`.

## Simplify `Directory` and remove the `Tree` and `Forest` types

The `Directory` type represents the file system view of a snapshot. Its field
`sub_directories` is defined a `Forest` based on `Tree`. The types are
over-engineered from such a simple concept. We could refactor `Directory` to
use `DirectoryContents` for its items and not to use `Tree` or `Forest` at all.

We also found the `list_directory()` method duplicates with `iter()` method.
Hence `list_directory()` is removed, together with `SystemType` type.

## The new API

With the changes proposed in the previous section, we describe what the new API
would look like and how they meet the requirements.

### Common principles

#### How to identify things that resolve into a commit

In our API, it will be good to have a single way to identify all refs and
objects that resolve into commits. In other words, we try to avoid using
different ways at different places. Currently there are multiple types in the
API for this purpose:

- Commit
- Oid
- Rev

Because `Rev` is the most high level among those and supports refs already,
I think we should use `Rev` in our API as much as possible.

#### How to identify History

TBD

### Code browsing

The user should be able to browse the files and directories for any given
commits or references. The core API is:

- Create a root Directory:
```Rust
imp RepositoryRef {
    pub fn snapshot(&self, rev: &Rev) -> Result<Directory, Error>;
}
```

- Browse a Directory:
```Rust
impl Directory {
    pub fn contents(&self) -> impl Iterator<Item = &DirectoryContents>;
}
```
where `DirectoryContents` supports both files and sub-directories:
```Rust
pub enum DirectoryContents {
    /// The `File` variant contains the file's name and the [`File`] itself.
    File {
        /// The name of the file.
        name: Label,
        /// The file data.
        file: File,
    },
    /// The `Directory` variant contains a sub-directory to the current one.
    Directory(Directory),
}
```

### Diffs

The user would be able to create a diff between any two revisions that resolve
into two commits.

The main change is to use `Rev` instead of `Oid` to identify `from` and `to`.
The core API is:

```Rust
imp RepositoryRef {
    pub fn diff(&self, from: &Rev, to: &Rev) -> Result<Diff, Error>;
}
```

To help convert from `Oid` to `Rev`, we provide a helper method:
```Rust
imp RepositoryRef {
    /// Returns the Oid of `rev`.
    pub fn rev_oid(&self, rev: &Rev) -> Result<Oid, Error>;
}
```

## Error handling

TBD
