## Version 0.9.0

This release consists of a major rewrite of this crate. Its API is overall
simplified and is not compatible with the previous version (v0.8.0). The main
changes include:

- `Browser` is removed. Its methods are implemented directly with `Repository`.
- Git will be the only supported VCS. Any extension points for other VCSes were
removed.
- `Ref` and `RefScope` are removed. Re-use the `git-ref-format` crate and a new
`Glob` type for the refspec patterns.
- Added support of `Tree` and `Blob` that correspond to their definitions in
Git.
- Added two new traits `Revision` and `ToCommit` that make methods flexible and
still simple to use.

For more details, please check out the crate's documentation.
