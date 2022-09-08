# radicle-git

The home for all Git utilities related to Radicle. While the majority
of the utilities will attempt to be general, there may be more
opinionated parts that are geared towards Radicle.

## Motivation

The [git2] and [gitoxide] libraries provide low-level APIs for
interacting with git repositories and the git protocol. This family of
crates attempts to provide a higher-level wrapper around those to
provide a more useful API for the Radicle protocol and UI clients
building on top of the protocol.

## Overview

The repository is defined as a series of crates:

* `git-ext` -- provides higher-level types over common types found in `git2`.
* `git-ref-format` -- provides a higher-level API for constructing git.
  reference paths. Prefer this over `git-ext`'s `RefLike` and `RefspecPattern`.
* `git-trailers` -- provides a way to parse and construct git trailers.
* `git-types` -- provides higher-level types, e.g. `Namespace`,
  `Reference`, `Remote`, etc.
* `link-git` -- provides a higher-level API for git's `refdb`, `odb`,
  and the git protocol.
* `macros` -- provides macros for the `git-ext` references types.
* `std-ext` -- provides some utilities extending the standard library.
* `test` -- a shim crate that refers depends on all other, individual
  test crates.

## Tests

Please refer to [test/README.md][test] for understanding how our tests
are organised.

## Contribute

Please read [CONTRIBUTING.md][contrib] for a guide on contributing to
this repository.

## Credits

Thanks to the previous maintainers of the `radicle-link` repository,
Kim, Alex, and Fintan, for providing the foundation to work upon -- as
well as the [contributors][link-contributors].

[contrib]: ./CONTRIBUTING.md
[git2]: https://github.com/rust-lang/git2-rs
[gitoxide]: https://github.com/Byron/gitoxide
[link-contributors]: https://github.com/radicle-dev/radicle-link/graphs/contributors
[test]: ./test/README.md
