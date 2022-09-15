// Copyright © 2019-2020 The Radicle Foundation <hello@radicle.foundation>
//
// This file is part of radicle-link, distributed under the GPLv3 with Radicle
// Linking Exception. For full terms see the included LICENSE file.

use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    str::FromStr,
};

use either::Either;

use git_ext::{error::is_not_found_err, Oid};
use git_ref_format::{Qualified, RefStr, RefString};
use std_ext::Void;

use crate::{
    config::{self, Config},
    odb,
    refdb::{
        self,
        resolve,
        write::{previous, Applied, Policy, SymrefTarget, Update, Updated},
        Read as _,
        Reference,
        Target,
    },
    signature::UserInfo,
};

use super::read::{self, Read};

pub mod error;

/// A read-write storage backend for accessing git's odb and refdb.
///
/// For read-only access to the odb see [`odb::Read`].
/// For write access to the odb see [`odb::Write`].
///
/// For read-only access to the refdb see [`refdb::Read`].
/// For write access to the refdb see [`refdb::Write`].
///
/// To construct the `Write` storage use [`Read::open`].
pub struct Write<Owner> {
    inner: Read,
    owner: Owner,
    info: UserInfo,
}

impl<O> Write<O>
where
    O: config::Owner,
    <O::RadIdentifier as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    /// Open the [`Write`] storage, initialising it if it doesn't exist.
    ///
    /// Note that a [`Write`] is tied to the [`config::Owner`] with which it was
    /// initialised, attempting to open it with a different one (that is, a
    /// different owner) will return an error.
    ///
    /// # Concurrency
    ///
    /// [`Write`] can be sent between threads, but it can't be shared between
    /// threads. _Some_ operations are safe to perform concurrently in much
    /// the same way two `git` processes can access the same repository.
    /// However, if you need multiple [`Write`]s to be shared between
    /// threads, use a [`crate::Pool`] instead.
    pub fn open<P: AsRef<Path>>(path: P, info: UserInfo, owner: O) -> Result<Self, error::Init> {
        crate::init();

        let path = path.as_ref();
        let raw = match git2::Repository::open_bare(path) {
            Err(e) if is_not_found_err(&e) => {
                let mut backend = git2::Repository::init_opts(
                    path,
                    git2::RepositoryInitOptions::new()
                        .bare(true)
                        .no_reinit(true)
                        .external_template(false),
                )?;
                Config::init(&mut backend, &owner, info.clone())?;

                Ok(backend)
            },
            Ok(repo) => Ok(repo),
            Err(e) => Err(e),
        }?;

        let config = Config::try_from(&raw)?;
        let actual = config.rad_identifier::<O::RadIdentifier>()?;
        let current = owner.rad_identifier();

        if actual != current {
            return Err(error::Init::OwnerMismatch {
                current: current.to_string(),
                actual: actual.to_string(),
            });
        }

        Ok(Self {
            inner: Read { raw },
            owner,
            info,
        })
    }

    /// Get the underlying [`Config`] of the [`Write`] storage.
    pub fn config(&self) -> Result<Config<O>, config::Error> {
        Config::try_from(self)
    }

    /// Get the read-only [`Config`] of the [`Write`] storage.
    pub fn config_readonly(&self) -> Result<Config<PhantomData<Void>>, git2::Error> {
        Config::try_from(&self.inner.raw)
    }

    pub(crate) fn config_path(&self) -> PathBuf {
        config::path(&self.inner.raw)
    }
}

impl<O> Write<O> {
    /// Return a read-only handle of the storage.
    pub fn read_only(&self) -> &Read {
        &self.inner
    }

    /// Return the owner of the storage.
    pub fn owner(&self) -> &O {
        &self.owner
    }

    /// Return the [`UserInfo`] of the storage.
    pub fn info(&self) -> &UserInfo {
        &self.info
    }

    fn as_raw(&self) -> &git2::Repository {
        &self.inner.raw
    }
}

// refdb impls

impl<'a, S> refdb::Read for &'a Write<S> {
    type FindRef = <&'a Read as refdb::Read>::FindRef;
    type FindRefs = <&'a Read as refdb::Read>::FindRefs;
    type FindRefOid = <&'a Read as refdb::Read>::FindRefOid;

    type References = <&'a Read as refdb::Read>::References;

    fn find_reference<Ref>(&self, reference: Ref) -> Result<Option<Reference>, Self::FindRef>
    where
        Ref: AsRef<git_ref_format::RefStr>,
    {
        self.read_only().find_reference(reference)
    }

    fn find_references<Pat>(&self, reference: Pat) -> Result<Self::References, Self::FindRefs>
    where
        Pat: AsRef<git_ref_format::refspec::PatternStr>,
    {
        self.read_only().find_references(reference)
    }

    fn find_reference_oid<Ref>(&self, reference: Ref) -> Result<Option<Oid>, Self::FindRefOid>
    where
        Ref: AsRef<git_ref_format::RefStr>,
    {
        self.read_only().find_reference_oid(reference)
    }
}

impl<'a, S> refdb::Write for &'a Write<S> {
    type UpdateError = error::Update;

    fn update<'b, U>(&mut self, updates: U) -> Result<refdb::write::Applied<'b>, Self::UpdateError>
    where
        U: IntoIterator<Item = Update<'b>>,
    {
        let mut refdb = Transaction::new(self)?;
        let mut applied = Applied::default();
        for up in updates.into_iter() {
            match up {
                Update::Direct {
                    name,
                    target,
                    no_ff,
                    previous,
                    reflog,
                } => match refdb.direct(name, target, no_ff, previous, reflog)? {
                    Either::Left(update) => applied.rejected.push(update),
                    Either::Right(updated) => applied.updated.push(updated),
                },
                Update::Symbolic {
                    name,
                    target,
                    type_change,
                    previous,
                    reflog,
                } => match refdb.symbolic(name, target, type_change, previous, reflog)? {
                    Either::Left(update) => applied.rejected.push(update),
                    Either::Right(updated) => applied.updated.extend(updated),
                },
                Update::Remove { name, previous } => match refdb.remove(name, previous)? {
                    Either::Left(update) => applied.rejected.push(update),
                    Either::Right(updated) => applied.updated.push(updated),
                },
            }
        }
        refdb.commit()?;

        Ok(applied)
    }
}

/// An internal struct combining a [`Write`] and [`git2::Transaction`].
// TODO: include optional namespace
struct Transaction<'a, O> {
    refdb: &'a Write<O>,
    txn: git2::Transaction<'a>,
}

impl<'a, S> Transaction<'a, S> {
    pub fn new(refdb: &'a Write<S>) -> Result<Self, git2::Error> {
        let txn = refdb.inner.raw.transaction()?;
        Ok(Self { refdb, txn })
    }

    /// Perform an [`Update::Direct`] within the [`Transaction`].
    ///
    /// Steps:
    /// 1. Get the state of the reference
    ///   a. if it did not exist create the source and destination references,
    ///      skip the next steps.
    ///   b. if it did, then follow the next steps.
    ///
    /// 2. Guard against the [`previous::Edit`] value, if this fails then reject
    /// the [`Update`].
    ///
    /// 3. Check the fast-forward policy, either aborting,
    /// rejecting, or accepting the update
    pub fn direct<'b>(
        &mut self,
        name: Qualified<'b>,
        target: Oid,
        no_ff: Policy,
        previous: previous::Edit,
        reflog: String,
    ) -> Result<Either<Update<'b>, Updated>, error::Update> {
        let prev = self.refdb.find_reference(&name)?;
        let given = prev
            .as_ref()
            .map(|prev| resolve(self.refdb.as_raw(), prev))
            .transpose()?;
        if let Err(_err) = previous.guard(given) {
            return Ok(Either::Left(Update::Direct {
                name,
                target,
                no_ff,
                previous,
                reflog,
            }));
        }

        let not_ff = match given {
            Some(prev) => {
                if !self.is_ff(&name, target, prev)? {
                    Some(prev)
                } else {
                    None
                }
            },
            None => None,
        };

        match not_ff {
            // It wasn't an fast-forward so we check our policy
            Some(cur) => match no_ff {
                Policy::Abort => Err(error::Update::NonFF {
                    name: name.into(),
                    new: target,
                    cur,
                }),
                Policy::Reject => Ok(Either::Left(Update::Direct {
                    name,
                    target,
                    no_ff,
                    previous,
                    reflog,
                })),
                Policy::Allow => Ok(Either::Right(self.direct_edit(&name, target, &reflog)?)),
            },
            // It was an fast-forward so we go ahead and make the edit
            None => Ok(Either::Right(self.direct_edit(&name, target, &reflog)?)),
        }
    }

    /// Perform an [`Update::Symbolic`] within the [`Transaction`].
    ///
    /// Steps:
    /// 1. Get the state of the source reference
    ///   a. if it did not exist create the source and destination references,
    ///      skip the next steps.
    ///   b. if it did, then follow the next steps.
    ///
    /// 2. Guard against the `type_change`, aborting or rejecting depending on
    /// the [`Policy`].
    ///
    /// 3. Get the state of the destination reference.
    ///
    /// 4. Check the target of the desitination:
    ///   a. if it's direct then make the edit depending on the fast-forward
    ///      status.
    ///   b. if it's symbolic then this is an error.
    pub fn symbolic<'b>(
        &mut self,
        name: Qualified<'b>,
        target: SymrefTarget<'b>,
        type_change: Policy,
        previous: previous::Edit,
        reflog: String,
    ) -> Result<Either<Update<'b>, Vec<Updated>>, error::Update> {
        let src = self.refdb.find_reference(&name)?;
        match src {
            Some(src) => match src.target {
                Target::Direct { .. } if matches!(type_change, Policy::Abort) => {
                    Err(error::Update::TypeChange(name.into()))
                },
                Target::Direct { .. } if matches!(type_change, Policy::Reject) => {
                    Ok(Either::Left(Update::Symbolic {
                        name,
                        target,
                        type_change,
                        previous,
                        reflog,
                    }))
                },
                _ => {
                    let dst = self.refdb.find_reference(&target.name)?;
                    match dst {
                        Some(dst) => match dst.target {
                            Target::Direct { oid: dst } => {
                                let is_ff = target.target != dst
                                    && self.is_ff(&target.name, target.target, dst)?;
                                Ok(Either::Right(
                                    self.symbolic_edit(name, target, &reflog, is_ff)?,
                                ))
                            },
                            Target::Symbolic { .. } => Err(error::Update::TargetSymbolic(dst.name)),
                        },
                        None => Ok(Either::Right(
                            self.symbolic_edit(name, target, &reflog, true)?,
                        )),
                    }
                },
            },
            None => Ok(Either::Right(
                self.symbolic_edit(name, target, &reflog, true)?,
            )),
        }
    }

    /// Perform an [`Update::Remove`] within the [`Transaction`].
    ///
    /// Steps:
    /// 1. Get the state of the reference
    ///
    /// 2. Guard against the [`previous::Remove`] value, if this fails then
    /// reject the [`Update`].
    ///
    /// 3. Remove the reference
    ///
    /// # Panics
    ///
    /// The `previous` SHOULD guard against the reference not existing, so this
    /// will panic if the previous reference was missing AND passed the
    /// `previous::guard`.
    pub fn remove<'b>(
        &mut self,
        name: Qualified<'b>,
        previous: previous::Remove,
    ) -> Result<Either<Update<'b>, Updated>, error::Update> {
        let prev = self.refdb.find_reference(&name)?;
        let given = prev
            .as_ref()
            .map(|prev| resolve(self.refdb.as_raw(), prev))
            .transpose()?;
        if let Err(_err) = previous.guard(given) {
            Ok(Either::Left(Update::Remove { name, previous }))
        } else {
            match given {
                None => {
                    panic!("BUG: the previous value for a reference to be removed was not given, but its existence SHOULD be guarded")
                },
                Some(previous) => Ok(Either::Right(self.remove_edit(name, previous)?)),
            }
        }
    }

    pub fn lock<R>(&mut self, reference: R) -> Result<(), error::Transaction>
    where
        R: AsRef<RefStr>,
    {
        let reference = reference.as_ref();
        self.txn
            .lock_ref(reference.as_str())
            .map_err(|err| error::Transaction::Lock {
                reference: reference.to_owned(),
                source: err,
            })
    }

    pub fn commit(self) -> Result<(), error::Transaction> {
        self.txn
            .commit()
            .map_err(|err| error::Transaction::Commit { source: err })
    }

    pub fn direct_edit<R>(
        &mut self,
        reference: R,
        target: Oid,
        reflog: &str,
    ) -> Result<Updated, error::Transaction>
    where
        R: AsRef<RefStr>,
    {
        let reference = reference.as_ref();
        self.lock(reference)?;
        let info = self.refdb.info();
        let sig = info
            .signature()
            .map_err(|err| error::Transaction::Signature {
                name: info.name.to_owned(),
                email: info.email.to_owned(),
                source: err,
            })?;
        self.txn
            .set_target(reference.as_str(), target.into(), Some(&sig), reflog)
            .map_err(|err| error::Transaction::SetDirect {
                reference: reference.to_owned(),
                target,
                source: err,
            })?;

        Ok(Updated::Direct {
            name: reference.to_owned(),
            target,
        })
    }

    pub fn symbolic_edit<R>(
        &mut self,
        reference: R,
        target: SymrefTarget,
        reflog: &str,
        is_ff: bool,
    ) -> Result<Vec<Updated>, error::Transaction>
    where
        R: AsRef<RefStr>,
    {
        let reference = reference.as_ref();
        self.lock(reference)?;
        self.lock(&target.name)?;

        let SymrefTarget { name: dst, target } = target;

        let mut edits = Vec::with_capacity(2);
        let info = self.refdb.info();
        let sig = info
            .signature()
            .map_err(|err| error::Transaction::Signature {
                name: info.name.to_owned(),
                email: info.email.to_owned(),
                source: err,
            })?;
        if is_ff {
            let direct = self.direct_edit(&dst, target, reflog)?;
            edits.push(direct);
        }

        self.txn
            .set_symbolic_target(reference.as_str(), dst.as_str(), Some(&sig), reflog)
            .map_err(|err| error::Transaction::SetSymbolic {
                reference: reference.to_owned(),
                target: dst.clone().into(),
                source: err,
            })?;
        edits.push(Updated::Symbolic {
            name: reference.to_owned(),
            target: dst.into(),
        });
        Ok(edits)
    }

    pub fn remove_edit<R>(
        &mut self,
        reference: R,
        previous: Oid,
    ) -> Result<Updated, error::Transaction>
    where
        R: AsRef<RefStr>,
    {
        let reference = reference.as_ref();
        self.lock(reference)?;
        self.txn
            .remove(reference.as_str())
            .map_err(|err| error::Transaction::Remove {
                reference: reference.to_owned(),
                source: err,
            })?;
        Ok(Updated::Removed {
            name: reference.to_owned(),
            previous,
        })
    }

    fn is_ff<R>(&self, name: R, target: Oid, prev: Oid) -> Result<bool, error::Transaction>
    where
        R: AsRef<RefStr>,
    {
        self.refdb
            .inner
            .raw
            .graph_descendant_of(target.into(), prev.into())
            .map_err(|err| error::Transaction::Ancestry {
                name: name.as_ref().to_owned(),
                new: target,
                old: prev,
                source: err,
            })
    }
}

// odb impls

impl<S> odb::Read for Write<S> {
    type FindObj = <Read as odb::Read>::FindObj;
    type FindBlob = <Read as odb::Read>::FindBlob;
    type FindCommit = <Read as odb::Read>::FindCommit;
    type FindTag = <Read as odb::Read>::FindTag;
    type FindTree = <Read as odb::Read>::FindTree;

    fn find_object(&self, oid: Oid) -> Result<Option<crate::Object>, Self::FindObj> {
        self.read_only().find_object(oid)
    }

    fn find_blob(&self, oid: Oid) -> Result<Option<git2::Blob>, Self::FindBlob> {
        self.read_only().find_blob(oid)
    }

    fn find_commit(&self, oid: Oid) -> Result<Option<git2::Commit>, Self::FindCommit> {
        self.read_only().find_commit(oid)
    }

    fn find_tag(&self, oid: Oid) -> Result<Option<git2::Tag>, Self::FindTag> {
        self.read_only().find_tag(oid)
    }

    fn find_tree(&self, oid: Oid) -> Result<Option<git2::Tree>, Self::FindTree> {
        self.read_only().find_tree(oid)
    }
}

impl<S> odb::Write for Write<S> {
    type WriteBlob = git2::Error;
    type WriteCommit = git2::Error;
    type WriteTag = git2::Error;
    type WriteTree = git2::Error;

    fn write_blob(&self, data: &[u8]) -> Result<Oid, Self::WriteBlob> {
        self.as_raw().blob(data).map(Oid::from)
    }

    fn write_commit<'b>(
        &self,
        tree: &odb::Tree,
        parents: &[&odb::Commit<'b>],
        message: &str,
    ) -> Result<Oid, Self::WriteCommit> {
        let author = self.info.signature()?;
        self.as_raw()
            .commit(None, &author, &author, message, tree, parents)
            .map(Oid::from)
    }

    fn write_tag<R>(
        &self,
        name: R,
        target: &odb::Object,
        message: &str,
    ) -> Result<Oid, Self::WriteTag>
    where
        R: AsRef<RefStr>,
    {
        let tagger = self.info.signature()?;
        self.as_raw()
            .tag_annotation_create(name.as_ref().as_str(), target, &tagger, message)
            .map(Oid::from)
    }

    fn write_tree(&self, builder: git2::TreeBuilder) -> Result<Oid, Self::WriteTree> {
        builder.write().map(Oid::from)
    }
}
