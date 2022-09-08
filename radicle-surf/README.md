# radicle-surf

A code surfing library for VCS file systems 🏄‍♀️🏄‍♂️

Welcome to `radicle-surf`!

`radicle-surf` is a system to describe a file-system in a VCS world.
We have the concept of files and directories, but these objects can change over time while people iterate on them.
Thus, it is a file-system within history and we, the user, are viewing the file-system at a particular snapshot.
Alongside this, we will wish to take two snapshots and view their differences.

## Contributing

To get started on contributing you can check out our [developing guide](../DEVELOPMENT.md), and also
our [LICENSE](../LICENSE) file.

## The Community

Join our community disccussions at [radicle.community](https://radicle.community)!

# Example

To a taste for the capabilities of `radicle-surf` we provide an example below, but we also
keep our documentation and doc-tests up to date.

```rust
use radicle_surf::vcs::git;
use radicle_surf::file_system::{Label, Path, SystemType};
use radicle_surf::file_system::unsound;
use pretty_assertions::assert_eq;
use std::str::FromStr;

// We're going to point to this repo.
let repo = git::Repository::new("./data/git-platinum")?;

// Here we initialise a new Broswer for a the git repo.
let mut browser = git::Browser::new(&repo, "master")?;

// Set the history to a particular commit
let commit = git::Oid::from_str("80ded66281a4de2889cc07293a8f10947c6d57fe")?;
browser.commit(commit)?;

// Get the snapshot of the directory for our current HEAD of history.
let directory = browser.get_directory()?;

// Let's get a Path to the memory.rs file
let memory = unsound::path::new("src/memory.rs");

// And assert that we can find it!
assert!(directory.find_file(memory).is_some());

let root_contents = directory.list_directory();

assert_eq!(root_contents, vec![
    SystemType::file(unsound::label::new(".i-am-well-hidden")),
    SystemType::file(unsound::label::new(".i-too-am-hidden")),
    SystemType::file(unsound::label::new("README.md")),
    SystemType::directory(unsound::label::new("bin")),
    SystemType::directory(unsound::label::new("src")),
    SystemType::directory(unsound::label::new("text")),
    SystemType::directory(unsound::label::new("this")),
]);

let src = directory
    .find_directory(Path::new(unsound::label::new("src")))
    .expect("failed to find src");
let src_contents = src.list_directory();

assert_eq!(src_contents, vec![
    SystemType::file(unsound::label::new("Eval.hs")),
    SystemType::file(unsound::label::new("Folder.svelte")),
    SystemType::file(unsound::label::new("memory.rs")),
]);
```
