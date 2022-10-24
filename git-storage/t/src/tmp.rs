use git2::Repository;
use git_storage::{signature::UserInfo, Write};
use test_helpers::tempdir::WithTmpDir;

pub type TmpWriter = WithTmpDir<Write>;
pub type TmpRepo = WithTmpDir<Repository>;

pub fn writer(user: UserInfo) -> TmpWriter {
    WithTmpDir::new(|path| {
        Write::open(path, user)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e)))
    })
    .unwrap()
}
