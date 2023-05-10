use std::{io, str::FromStr as _, string::ToString as _};

use radicle_git_ext::{
    author::{self, Author},
    commit::{headers::Headers, trailers::OwnedTrailer, Commit},
};
use test_helpers::tempdir::WithTmpDir;

const NO_TRAILER: &str = "\
tree 50d6ef440728217febf9e35716d8b0296608d7f8
parent 0ad95dbdfe9fdf81938ca419cf740469173e2022
parent a4ec9e07e1b2e6f37f7119651ae3bb63b79988b6
author Fintan Halpenny <fintan.halpenny@gmail.com> 1669292989 +0000
committer Fintan Halpenny <fintan.halpenny@gmail.com> 1669292989 +0000

Merge remote-tracking branch 'origin/surf/organise-tests'

* origin/surf/organise-tests:
  radicle-surf: organise tests
";

const SINGLE_TRAILER: &str = "\
tree 50d6ef440728217febf9e35716d8b0296608d7f8
parent 0ad95dbdfe9fdf81938ca419cf740469173e2022
parent a4ec9e07e1b2e6f37f7119651ae3bb63b79988b6
author Fintan Halpenny <fintan.halpenny@gmail.com> 1669292989 +0000
committer Fintan Halpenny <fintan.halpenny@gmail.com> 1669292989 +0000

Merge remote-tracking branch 'origin/surf/organise-tests'

* origin/surf/organise-tests:
  radicle-surf: organise tests

Signed-off-by: Fintan Halpenny <fintan.halpenny@gmail.com>
";

const UNSIGNED: &str = "\
tree c66cc435f83ed0fba90ed4500e9b4b96e9bd001b
parent af06ad645133f580a87895353508053c5de60716
author Alexis Sellier <alexis@radicle.xyz> 1664467633 +0200
committer Alexis Sellier <alexis@radicle.xyz> 1664786099 -0200

Add SSH functionality with new `radicle-ssh`

We borrow code from `thrussh`, refactored to be runtime-less.

X-Signed-Off-By: Alex Sellier
X-Co-Authored-By: Fintan Halpenny
";

const SSH_SIGNATURE: &str = "\
-----BEGIN SSH SIGNATURE-----
U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgvjrQogRxxLjzzWns8+mKJAGzEX
4fm2ALoN7pyvD2ttQAAAADZ2l0AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5
AAAAQIQvhIewOgGfnXLgR5Qe1ZEr2vjekYXTdOfNWICi6ZiosgfZnIqV0enCPC4arVqQg+
GPp0HqxaB911OnSAr6bwU=
-----END SSH SIGNATURE-----";

const PGP_SIGNATURE: &str = "\
-----BEGIN PGP SIGNATURE-----
iQIzBAABCAAdFiEEHe7BWIo9taTY6TIiJVL7b2QGbLcFAmNcDhsACgkQJVL7b2QG
bLcc9Q//RgKf5N4enta9AuszGJZvdFhMPfIDUdw+WAZA6Z8zDPb/aAXZrPP/KIOM
zmX08FTqjP9B9YeWrEcFuAtxsRNqbDKrfpko9Y6bTsdrAJg3WIypBb9F8YDKJ6BO
CORJJqWOsLW129jW+mJDhcE0YTvPlcMiMI2qjVXKhU6Ag11W8IRZyTb9tvEaDjBR
YUnkPvgubv61K9BeUKexE2MakPBldaQtl0MF1Dk7/zo5btLd+KP0SOUKEhuMEu5b
LATHHdiYjt/2Xz7q8EcrFxXUaipxZe89dfTdi2ooJQw3ZDqjDHsGTHpDeBuzuSaJ
9fKVRwFz/78onfHPhmU4wfUhh+Fcl90p5/T+4dt2K6cr+7rq078e+aJYxkX2d0MG
PG0xGP0RN4g+X92K1kGuzoe4870xAnRTNh5nUB+X9snO8tVqQZTb0M2yI+sTsKrv
w/f+uiqL6e9DgIxlO5dgiNHCVoCs1QJ900jUGisrlzS4+n6GzMsG6s3c01X4yY9G
Ou/kGkMsn7tqejqC9RufygcchCFZqYwaHQwPkiYhfYGMarMpoCFvll0h8tSparpS
nnpAQXVdu8m3v1YdPUuTg5ksxSOe9HCIlVXGFhxy3iqCVRn+51FRnUI63rMTOm9/
LBqzvji02lDUPGqPgXfcCS0ty8FM2flBIXnwb8TDzCaPYhf53+U=
=6dw2
-----END PGP SIGNATURE-----";

const SIGNED: &str = "\
tree c66cc435f83ed0fba90ed4500e9b4b96e9bd001b
parent af06ad645133f580a87895353508053c5de60716
author Alexis Sellier <alexis@radicle.xyz> 1664467633 +0200
committer Alexis Sellier <alexis@radicle.xyz> 1664786099 -0200
other e6fe3c97619deb8ab4198620f9a7eb79d98363dd
gpgsig -----BEGIN SSH SIGNATURE-----
 U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgvjrQogRxxLjzzWns8+mKJAGzEX
 4fm2ALoN7pyvD2ttQAAAADZ2l0AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5
 AAAAQIQvhIewOgGfnXLgR5Qe1ZEr2vjekYXTdOfNWICi6ZiosgfZnIqV0enCPC4arVqQg+
 GPp0HqxaB911OnSAr6bwU=
 -----END SSH SIGNATURE-----
gpgsig -----BEGIN PGP SIGNATURE-----
 iQIzBAABCAAdFiEEHe7BWIo9taTY6TIiJVL7b2QGbLcFAmNcDhsACgkQJVL7b2QG
 bLcc9Q//RgKf5N4enta9AuszGJZvdFhMPfIDUdw+WAZA6Z8zDPb/aAXZrPP/KIOM
 zmX08FTqjP9B9YeWrEcFuAtxsRNqbDKrfpko9Y6bTsdrAJg3WIypBb9F8YDKJ6BO
 CORJJqWOsLW129jW+mJDhcE0YTvPlcMiMI2qjVXKhU6Ag11W8IRZyTb9tvEaDjBR
 YUnkPvgubv61K9BeUKexE2MakPBldaQtl0MF1Dk7/zo5btLd+KP0SOUKEhuMEu5b
 LATHHdiYjt/2Xz7q8EcrFxXUaipxZe89dfTdi2ooJQw3ZDqjDHsGTHpDeBuzuSaJ
 9fKVRwFz/78onfHPhmU4wfUhh+Fcl90p5/T+4dt2K6cr+7rq078e+aJYxkX2d0MG
 PG0xGP0RN4g+X92K1kGuzoe4870xAnRTNh5nUB+X9snO8tVqQZTb0M2yI+sTsKrv
 w/f+uiqL6e9DgIxlO5dgiNHCVoCs1QJ900jUGisrlzS4+n6GzMsG6s3c01X4yY9G
 Ou/kGkMsn7tqejqC9RufygcchCFZqYwaHQwPkiYhfYGMarMpoCFvll0h8tSparpS
 nnpAQXVdu8m3v1YdPUuTg5ksxSOe9HCIlVXGFhxy3iqCVRn+51FRnUI63rMTOm9/
 LBqzvji02lDUPGqPgXfcCS0ty8FM2flBIXnwb8TDzCaPYhf53+U=
 =6dw2
 -----END PGP SIGNATURE-----

Add SSH functionality with new `radicle-ssh`

We borrow code from `thrussh`, refactored to be runtime-less.

X-Signed-Off-By: Alex Sellier
X-Co-Authored-By: Fintan Halpenny
";

#[test]
fn test_push_header() {
    let mut commit = Commit::from_str(UNSIGNED).unwrap();

    commit.push_header("other", "e6fe3c97619deb8ab4198620f9a7eb79d98363dd");
    commit.push_header("gpgsig", SSH_SIGNATURE);
    commit.push_header("gpgsig", PGP_SIGNATURE);

    assert_eq!(commit.to_string(), SIGNED);
}

#[test]
fn test_get_header() {
    let commit = Commit::from_str(SIGNED).unwrap();

    assert_eq!(
        commit
            .signatures()
            .map(|sig| sig.to_string())
            .collect::<Vec<_>>(),
        vec![SSH_SIGNATURE.to_owned(), PGP_SIGNATURE.to_owned()]
    );
    assert_eq!(
        commit.values("other").collect::<Vec<_>>(),
        vec![String::from("e6fe3c97619deb8ab4198620f9a7eb79d98363dd")],
    );
    assert!(commit.values("unknown").next().is_none());
}

#[test]
fn test_conversion() {
    assert_eq!(
        Commit::from_str(NO_TRAILER).unwrap().to_string(),
        NO_TRAILER
    );
    assert_eq!(
        Commit::from_str(SINGLE_TRAILER).unwrap().to_string(),
        SINGLE_TRAILER
    );
    assert_eq!(Commit::from_str(SIGNED).unwrap().to_string(), SIGNED);
    assert_eq!(Commit::from_str(UNSIGNED).unwrap().to_string(), UNSIGNED);
}

#[test]
fn valid_commits() {
    let radicle_git = format!(
        "file://{}",
        git2::Repository::discover(".").unwrap().path().display()
    );
    let repo = WithTmpDir::new(|path| {
        let repo = git2::Repository::clone(&radicle_git, path)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        Ok::<_, io::Error>(repo)
    })
    .unwrap();

    let mut walk = repo.revwalk().unwrap();
    walk.push_head().unwrap();

    // take the first 20 commits and make sure we can parse them
    for oid in walk.take(20) {
        let oid = oid.unwrap();
        let commit = Commit::read(&repo, oid);
        assert!(commit.is_ok(), "Oid: {oid}, Error: {commit:?}")
    }
}

#[test]
fn write_valid_commit() {
    let repo = WithTmpDir::new(|path| {
        git2::Repository::init(path).map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    })
    .unwrap();
    let author = Author {
        name: "Terry".to_owned(),
        email: "terry.pratchett@proton.mail".to_owned(),
        time: author::Time::new(0, 0),
    };
    let blob = repo.blob(b"The Colour of Magic").unwrap();
    let mut tb = repo.treebuilder(None).unwrap();
    tb.insert("The Colour of Magic", blob, git2::FileMode::Blob.into())
        .unwrap();
    let tree = tb.write().unwrap();
    let commit = repo
        .commit(
            None,
            &git2::Signature::try_from(&author).unwrap(),
            &git2::Signature::try_from(&author).unwrap(),
            "New beginnings",
            &repo.find_tree(tree).unwrap(),
            &[],
        )
        .unwrap();

    let headers = Headers::new();
    let message = "Write Discworld".to_owned();

    let invalid = Commit::new(
        tree,
        vec![blob],
        author.clone(),
        author.clone(),
        headers.clone(),
        message.clone(),
        None::<OwnedTrailer>,
    )
    .write(&repo);
    assert!(invalid.is_err());

    let invalid = Commit::new(
        blob,
        vec![commit],
        author.clone(),
        author.clone(),
        headers.clone(),
        message.clone(),
        None::<OwnedTrailer>,
    )
    .write(&repo);
    assert!(invalid.is_err());

    let valid = Commit::new(
        tree,
        vec![commit],
        author.clone(),
        author,
        headers,
        message,
        None::<OwnedTrailer>,
    )
    .write(&repo);
    assert!(valid.is_ok())
}

#[test]
fn author_roundtrip() {
    let author = "author Fintan Halpenny <fintan.halpenny@gmail.com> 1669292989 +0000";
    assert_eq!(
        author.parse::<Author>().unwrap().to_string(),
        author.to_string()
    );

    let author = "author Alexis Sellier <alexis@radicle.xyz> 1664467633 +0200";
    assert_eq!(
        author.parse::<Author>().unwrap().to_string(),
        author.to_string()
    );

    let author = "author Alexis Sellier <alexis@radicle.xyz> 1664467633 -0200";
    assert_eq!(
        author.parse::<Author>().unwrap().to_string(),
        author.to_string()
    );
}
