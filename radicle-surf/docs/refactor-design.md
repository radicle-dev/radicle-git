# An updated design for radicle-surf

Now we have ported the `radicle-surf` crate from its own github repo to be part of the `radicle-git` repo. We are taking this opportunity to refactor its design as well. Intuitively, `radicle-surf` provides an API so that one can use it to create a github-like UI for a git repo:

- Given a commit (or other types of ref), list the content: i.e. files and directories.
- Generate a diff between two commits.
- List the history of the commits.
- List refs: Branches, Tags.

The main goals of the changes are:

- make API simpler whenever possible.
- address open issues in the original `radicle-surf` repo as much as possible.
- not to shy away from being `git` specific. (i.e. not to consider supporting other VCS systems)

## Make API simpler

The current API has quite a bit accidental complexity that is not inherent with the requirements, especially when we can be git specific and don't care about other VCS systems.

### Remove the `Browser`

The type `Browser` is awkward as of today:

- it is not a source of truth of any information. For example, `list_branches` method is just a wrapper of `Repository::list_branches`.
- it takes in `History`, but really works at the `Snapshot` level.
- it is mutable but its state does not help much.

Can we just remove `Browser` and implement its functionalities using other types?

- For iteratoring the history, use `History`.
- For generating `Directory`, use `Repository` directly given a `Rev`.
- For accessing `Branch`, `Tag` or `Commit`, use `Repository`.

## Remove the `Snapshot`

A `Snapshot` should be really just a tree (or `Directory`) of a `Commit` in git. Currently it is a function that returns a `Directory`. Because it is OK to be git specific, we don't need to have this generic function to create a snapshot across different VCS systems.

The only `snapshot` function defined currently:

```Rust
let snapshot = Box::new(|repository: &RepositoryRef<'a>, history: &History| {
            let tree = Self::get_tree(repository.repo_ref, history.0.first())?;
            Ok(directory::Directory::from_hash_map(tree))
        });
```

The above function can be easily implement as a method of `Repository`.
