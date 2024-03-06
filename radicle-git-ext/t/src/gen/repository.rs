use std::{fmt, fmt::Write, ops::Deref, rc::Rc, sync::Arc};

use git2::Oid;
use proptest::strategy::{Just, Strategy};
use radicle_git_ext::author::{Author, Time};

#[derive(Clone)]
pub struct GenRepository(Arc<git2::Repository>);

impl fmt::Debug for GenRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GenRepository")
            .field("path", &self.0.path().display())
            .finish()
    }
}

impl Deref for GenRepository {
    type Target = git2::Repository;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn author() -> impl Strategy<Value = Author> {
    alphanumeric().prop_flat_map(move |name| {
        (Just(name), alphanumeric()).prop_flat_map(|(name, domain)| {
            (Just(name), Just(domain), (0..1000i64)).prop_map(move |(name, domain, time)| {
                let email = format!("{name}@{domain}");
                Author {
                    name,
                    email,
                    time: Time::new(time, 0),
                }
            })
        })
    })
}

#[derive(Clone, Debug)]
pub struct Commit {
    author: Author,
    committer: Author,
    message: String,
    tree: Tree,
    parent: Option<Rc<Commit>>,
}

impl Commit {
    pub fn write(
        &self,
        repo: &git2::Repository,
        refname: &str,
        signature: Option<&str>,
    ) -> Result<Oid, git2::Error> {
        let oid = self.tree.write(repo)?;
        let tree = repo.find_tree(oid)?;
        let parent = match &self.parent {
            Some(parent) => {
                let parent = parent.write(repo, refname, signature)?;
                Some(repo.find_commit(parent)?)
            }
            None => None,
        };

        Ok(match signature {
            Some(signature) => {
                let content = self.content(tree.id(), parent.map(|p| p.id()));
                eprintln!("CONTENT:\n{content}\n");
                let oid = repo.commit_signed(&content, signature, None)?;
                repo.reference(refname, oid, true, "update reference")?;
                oid
            }
            None => repo.commit(
                Some(refname),
                &git2::Signature::try_from(&self.author)?,
                &git2::Signature::try_from(&self.committer)?,
                &self.message,
                &tree,
                &parent.iter().collect::<Vec<_>>(),
            )?,
        })
    }

    pub fn gen() -> impl Strategy<Value = Self> {
        let root = Self::data().prop_map(|(author, committer, message, tree)| Self {
            author,
            committer,
            message,
            tree,
            parent: None,
        });
        root.prop_recursive(10, 3, 10, |inner| {
            (Self::data(), inner).prop_map(|((author, committer, message, tree), parent)| Self {
                author,
                committer,
                message,
                tree,
                parent: Some(Rc::new(parent)),
            })
        })
    }

    fn data() -> impl Strategy<Value = (Author, Author, String, Tree)> {
        (author()).prop_flat_map(|a| {
            (Just(a), author()).prop_flat_map(|(author, committer)| {
                (Just(author), Just(committer), alphanumeric()).prop_flat_map(
                    |(author, committer, message)| {
                        (Just(author), Just(committer), Just(message), Tree::gen()).prop_map(
                            |(author, committer, message, tree)| (author, committer, message, tree),
                        )
                    },
                )
            })
        })
    }

    fn content(&self, tree: Oid, parent: Option<Oid>) -> String {
        let mut buf = String::new();

        writeln!(buf, "tree {}", tree).ok();

        if let Some(parent) = parent {
            writeln!(buf, "parent {parent}").ok();
        }

        writeln!(buf, "author {}", self.author).ok();
        writeln!(buf, "committer {}", self.committer).ok();

        writeln!(buf).ok();
        write!(buf, "{}", self.message.trim()).ok();
        writeln!(buf).ok();

        buf
    }
}

#[derive(Clone, Debug)]
pub enum Tree {
    Blob { name: String, data: String },
    Tree { name: String, inner: Vec<Tree> },
}

impl Tree {
    pub fn gen() -> impl Strategy<Value = Self> {
        let leaf = alphanumeric().prop_flat_map(|name| {
            (Just(name), alphanumeric()).prop_map(|(name, data)| Self::Blob { name, data })
        });
        leaf.prop_recursive(8, 64, 10, |inner| {
            (Just(inner), alphanumeric()).prop_flat_map(|(inner, name)| {
                (Just(name), proptest::collection::vec(inner, 1..10))
                    .prop_map(|(name, inner)| Self::Tree { name, inner })
            })
        })
    }

    pub fn write(&self, repo: &git2::Repository) -> Result<Oid, git2::Error> {
        let mut builder = repo.treebuilder(None)?;
        self.write_(repo, &mut builder)?;
        builder.write()
    }

    fn write_(
        &self,
        repo: &git2::Repository,
        builder: &mut git2::TreeBuilder,
    ) -> Result<Oid, git2::Error> {
        match self {
            Self::Blob { name, data } => {
                let oid = repo.blob(data.as_bytes())?;
                builder.insert(name, oid, git2::FileMode::Blob.into())?;
            }
            Self::Tree { name, inner } => {
                for data in inner {
                    let oid = data.write_(repo, builder)?;
                    builder.insert(name, oid, git2::FileMode::Tree.into())?;
                }
            }
        }
        builder.write()
    }
}

fn alphanumeric() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_]+"
}

pub fn armored_ssh() -> impl Strategy<Value = String> {
    "-----BEGIN SSH SIGNATURE-----\r?\n([A-Za-z0-9+/=\r\n]+)\r?\n-----END SSH SIGNATURE-----"
}
