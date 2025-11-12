use proptest::prelude::*;
use radicle_git_ext::ref_format::{check_ref_format, Error, Options};

use crate::git_ref_format::gen;

mod name;
mod pattern;

proptest! {
    #[test]
    fn disallow_onelevel(input in gen::trivial(), allow_pattern in any::<bool>()) {
        assert_matches!(
            check_ref_format(Options {
                    allow_onelevel: false,
                    allow_pattern,
                },
                &input
            ),
            Err(Error::OneLevel)
        )
    }
}
