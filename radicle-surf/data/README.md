# Updating [git-platinum][]

1. Push your changes to [`radicle-dev/git-platinum`][git-platinum] and/or update
   `surf/data/mock-branches.txt`.
2. Run `scripts/update-git-platinum.sh` from the repo root. This updates
   `surf/data/git-platinum.tgz`.
3. Run `cargo build` to unpack the updated repo.
4. Run the tests
5. Commit your changes. We provide a template below so that we can easily
   identify changes to `git-platinum`. Please fill in the details that follow a
   comment (`#`):
   ```
   data/git-platinum: # short reason for updating

   # provide a longer reason for making changes to git-platinum
   # as well as what has changed.
   ```



[git-platinum]: https://github.com/radicle-dev/git-platinum
