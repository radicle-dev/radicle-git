use std::path::Path;

use radicle_surf::history::History;

const GIT_PLATINUM: &str = "../data/git-platinum";

#[test]
pub fn test() {
    let repo = git2::Repository::open(GIT_PLATINUM)
        .expect("Could not retrieve ./data/git-platinum as git repository");
    let start = git2::Oid::from_str("3873745c8f6ffb45c990eb23b491d4b4b6182f95").unwrap();
    let history = History::file(&repo, start, Path::new("src/memory.rs").to_path_buf()).unwrap();

    for blob in history {
        match blob {
            Ok(blob_at) => {
                println!("================================================\n");
                println!("Commit: {}\n", blob_at.commit().id());
                println!("{}", std::str::from_utf8(blob_at.blob().content()).unwrap());
                println!("\n\n");
            },
            Err(err) => println!("Error: {}", err),
        }
    }

    assert!(false);
}
