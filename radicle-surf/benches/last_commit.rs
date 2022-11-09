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

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use radicle_surf::{
    file_system::{unsound, Path},
    vcs::git::{Branch, Repository},
};

fn last_commit_comparison(c: &mut Criterion) {
    let repo = Repository::new("./data/git-platinum")
        .expect("Could not retrieve ./data/git-platinum as git repository");
    let rev = Branch::local("master");

    let mut group = c.benchmark_group("Last Commit");
    for path in [
        Path::root(),
        unsound::path::new("~/src/memory.rs"),
        unsound::path::new("~/this/is/a/really/deeply/nested/directory/tree"),
    ]
    .iter()
    {
        group.bench_with_input(BenchmarkId::new("", path), path, |b, path| {
            b.iter(|| repo.as_ref().last_commit(path.clone(), &rev))
        });
    }
}

criterion_group!(benches, last_commit_comparison);
criterion_main!(benches);
