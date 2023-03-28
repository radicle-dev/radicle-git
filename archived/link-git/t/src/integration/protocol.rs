// Copyright © 2021 The Radicle Link Contributors
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

use std::{
    collections::BTreeSet,
    io,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

use bstr::ByteSlice as _;
use futures::{AsyncReadExt as _, TryFutureExt as _};
use link_git::protocol::{fetch, ls, packwriter, upload_pack, ObjectId, PackWriter, Ref};
use tempfile::{tempdir, TempDir};

fn upstream() -> TempDir {
    let tmp = tempdir().unwrap();

    let repo = git2::Repository::init_bare(&tmp).unwrap();
    let auth = git2::Signature::now("apollo", "apollo@cree.de").unwrap();

    let tree = {
        let empty = repo.treebuilder(None).unwrap();
        let oid = empty.write().unwrap();
        repo.find_tree(oid).unwrap()
    };
    let base = {
        let oid = repo
            .commit(
                Some("refs/namespaces/foo/refs/heads/main"),
                &auth,
                &auth,
                "initial",
                &tree,
                &[],
            )
            .unwrap();
        repo.find_commit(oid).unwrap()
    };
    let next = repo
        .commit(
            Some("refs/namespaces/foo/refs/heads/next"),
            &auth,
            &auth,
            "ng",
            &tree,
            &[&base],
        )
        .unwrap();
    repo.reference(
        "refs/namespaces/foo/refs/pulls/1/head",
        next,
        true,
        "pee arrr",
    )
    .unwrap();

    tmp
}

fn collect_refs(repo: &git2::Repository) -> Result<Vec<(String, git2::Oid)>, git2::Error> {
    repo.references()?
        .map(|x| x.map(|r| (r.name().unwrap().to_owned(), r.target().unwrap())))
        .collect()
}

fn update_tips<'a, T>(repo: &git2::Repository, tips: T) -> Result<(), anyhow::Error>
where
    T: IntoIterator<Item = &'a Ref>,
{
    for r in tips {
        match r {
            Ref::Direct { path, object } => {
                repo.reference(
                    path.to_str()?,
                    git2::Oid::from_bytes(object.as_slice())?,
                    true,
                    "",
                )?;
            },
            x => anyhow::bail!("unexpected ref variant: {:?}", x),
        }
    }

    Ok(())
}

fn collect_history(repo: &git2::Repository, tip: &str) -> Result<Vec<git2::Oid>, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_ref(tip)?;
    revwalk.collect()
}

fn run_ls_refs<R: AsRef<Path>>(remote: R, opt: ls::Options) -> io::Result<Vec<Ref>> {
    let (client, server) = futures_ringbuf::Endpoint::pair(256, 256);
    let client = async move {
        let (recv, send) = client.split();
        ls::ls_refs(opt, recv, send).await
    };
    let server = {
        let (recv, send) = server.split();
        upload_pack::upload_pack(&remote, recv, send).and_then(|(_hdr, run)| run)
    };

    let (client_out, server_out) =
        futures::executor::block_on(futures::future::try_join(client, server))?;
    assert!(server_out.success());
    Ok(client_out)
}

fn run_fetch<R, B, P>(
    remote: R,
    opt: fetch::Options,
    build_pack_writer: B,
) -> io::Result<fetch::Outputs<P::Output>>
where
    R: AsRef<Path>,
    B: FnOnce(Arc<AtomicBool>) -> P,
    P: PackWriter + Send + 'static,
    P::Output: Send + 'static,
{
    let (client, server) = futures_ringbuf::Endpoint::pair(256, 256);
    let client = async move {
        let (recv, send) = client.split();
        fetch::fetch(opt, build_pack_writer, recv, send).await
    };
    let server = {
        let (recv, send) = server.split();
        upload_pack::upload_pack(&remote, recv, send).and_then(|(_hdr, run)| run)
    };

    let (client_out, server_out) =
        futures::executor::block_on(futures::future::try_join(client, server))?;
    assert!(server_out.success());
    Ok(client_out)
}

#[test]
fn smoke() {
    let remote = upstream();
    let refs = run_ls_refs(
        &remote,
        ls::Options {
            repo: "foo".into(),
            extra_params: vec![],
            ref_prefixes: vec!["refs/heads/".into(), "refs/pulls/".into()],
        },
    )
    .unwrap();

    assert_eq!(
        refs.iter().map(|r| r.unpack().0).collect::<BTreeSet<_>>(),
        [
            "refs/heads/main".into(),
            "refs/heads/next".into(),
            "refs/pulls/1/head".into()
        ]
        .iter()
        .collect::<BTreeSet<_>>()
    );

    let out = run_fetch(
        &remote,
        fetch::Options {
            repo: "foo".into(),
            extra_params: vec![],
            haves: vec![],
            wants: vec![],
            want_refs: refs.iter().map(|r| r.unpack().0.clone()).collect(),
        },
        |_| packwriter::Discard,
    )
    .unwrap();

    assert!(out.pack.is_some());
}

#[test]
fn want_ref() {
    let remote = upstream();
    let out = run_fetch(
        &remote,
        fetch::Options {
            repo: "foo".into(),
            extra_params: vec![],
            haves: vec![],
            wants: vec![],
            want_refs: vec!["refs/heads/main".into(), "refs/pulls/1/head".into()],
        },
        |_| packwriter::Discard,
    )
    .unwrap();

    assert!(out.pack.is_some());
    assert_eq!(
        out.wanted_refs
            .iter()
            .map(|r| r.unpack().0)
            .collect::<BTreeSet<_>>(),
        ["refs/heads/main".into(), "refs/pulls/1/head".into(),]
            .iter()
            .collect::<BTreeSet<_>>()
    )
}

#[test]
#[should_panic(expected = "`fetch` is empty")]
fn empty_fetch() {
    let remote = upstream();
    run_fetch(
        &remote,
        fetch::Options {
            repo: "foo".into(),
            extra_params: vec![],
            haves: vec![],
            wants: vec![],
            want_refs: vec![],
        },
        |_| packwriter::Discard,
    )
    .unwrap();
}

fn clone_with<R, L, B, P>(remote: R, local: L, build_pack_writer: B)
where
    R: AsRef<Path>,
    L: AsRef<Path>,
    B: FnOnce(Arc<AtomicBool>) -> P,
    P: PackWriter + Send + 'static,
    P::Output: Send + 'static,
{
    let refs = run_ls_refs(
        &remote,
        ls::Options {
            repo: "foo".into(),
            extra_params: vec![],
            ref_prefixes: vec!["refs/heads/".into(), "refs/pulls/".into()],
        },
    )
    .unwrap();
    let out = run_fetch(
        &remote,
        fetch::Options {
            repo: "foo".into(),
            extra_params: vec![],
            haves: vec![],
            wants: vec![],
            want_refs: refs.iter().map(|r| r.unpack().0.clone()).collect(),
        },
        build_pack_writer,
    )
    .unwrap();

    assert!(out.pack.is_some());

    let remote_repo = git2::Repository::open(remote).unwrap();
    remote_repo.set_namespace("foo").unwrap();
    let local_repo = git2::Repository::open(&local).unwrap();

    update_tips(&local_repo, &out.wanted_refs).unwrap();

    let mut remote_refs = collect_refs(&remote_repo).unwrap();
    let mut local_refs = collect_refs(&local_repo).unwrap();

    remote_refs.sort();
    local_refs.sort();

    assert_eq!(remote_refs, local_refs);
}

#[test]
fn clone_libgit() {
    let remote = upstream();
    let local = tempdir().unwrap();
    let local_repo = git2::Repository::init(&local).unwrap();

    clone_with(&remote, &local, move |stop| {
        packwriter::Libgit::new(packwriter::Options::default(), local_repo, stop)
    })
}

#[test]
fn clone_gitoxide() {
    let remote = upstream();
    let local = tempdir().unwrap();
    let local_repo = git2::Repository::init(&local).unwrap();

    clone_with(&remote, &local, move |stop| {
        packwriter::Standard::new(
            local_repo.path(),
            packwriter::Options::default(),
            packwriter::StandardThickener::new(local_repo.path()),
            stop,
        )
    })
}

fn thin_pack_with<R, L, B, P>(remote: R, local: L, build_pack_writer: B)
where
    R: AsRef<Path>,
    L: AsRef<Path>,
    B: Fn(Arc<AtomicBool>) -> P,
    P: PackWriter + Send + 'static,
    P::Output: Send + 'static,
{
    // Clone main only
    {
        let out = run_fetch(
            &remote,
            fetch::Options {
                repo: "foo".into(),
                extra_params: vec![],
                haves: vec![],
                wants: vec![],
                want_refs: vec!["refs/heads/main".into()],
            },
            &build_pack_writer,
        )
        .unwrap();
        assert!(out.pack.is_some());
    }

    let remote_repo = git2::Repository::open(&remote).unwrap();
    remote_repo.set_namespace("foo").unwrap();
    let local_repo = git2::Repository::open(&local).unwrap();

    // Fetch next, which is ahead of main
    {
        let head = remote_repo.refname_to_id("refs/heads/main").unwrap();
        let out = run_fetch(
            &remote,
            fetch::Options {
                repo: "foo".into(),
                extra_params: vec![],
                haves: vec![ObjectId::from_20_bytes(head.as_bytes())],
                wants: vec![],
                want_refs: vec!["refs/heads/next".into()],
            },
            build_pack_writer,
        )
        .unwrap();
        assert!(out.pack.is_some());

        update_tips(&local_repo, &out.wanted_refs).unwrap();
    }

    let remote_history = collect_history(&remote_repo, "refs/heads/next").unwrap();
    let local_history = collect_history(&local_repo, "refs/heads/next").unwrap();

    assert!(!remote_history.is_empty());
    assert_eq!(remote_history, local_history)
}

#[test]
#[ignore]
fn thin_pack_libgit() {
    let remote = upstream();
    let local = tempdir().unwrap();

    thin_pack_with(&remote, &local, |stop| {
        let local_repo = git2::Repository::init(&local).unwrap();
        packwriter::Libgit::new(packwriter::Options::default(), local_repo, stop)
    })
}

#[test]
fn thin_pack_gitoxide() {
    let remote = upstream();
    let local = tempdir().unwrap();
    let local_repo = git2::Repository::init(&local).unwrap();
    let git_dir = local_repo.path().to_owned();

    thin_pack_with(&remote, &local, move |stop| {
        packwriter::Standard::new(
            &git_dir,
            packwriter::Options::default(),
            packwriter::StandardThickener::new(&git_dir),
            stop,
        )
    })
}
