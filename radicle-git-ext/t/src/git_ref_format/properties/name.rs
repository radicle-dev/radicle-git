use std::convert::TryFrom;

use proptest::prelude::*;
use radicle_git_ext::ref_format::{name, refname, Error, RefStr, RefString};
use test_helpers::roundtrip;

use crate::git_ref_format::gen;

proptest! {
    #[test]
    fn valid(input in gen::valid()) {
        assert_eq!(input.as_str(), RefStr::try_from_str(&input).unwrap().as_str())
    }

    #[test]
    fn invalid_char(input in gen::with_invalid_char()) {
        assert_matches!(RefString::try_from(input), Err(Error::InvalidChar(_)))
    }

    #[test]
    fn dot_lock(input in gen::ends_with_dot_lock()) {
        assert_matches!(RefString::try_from(input), Err(Error::DotLock))
    }

    #[test]
    fn double_dot(input in gen::with_double_dot()) {
        assert_matches!(RefString::try_from(input), Err(Error::DotDot))
    }

    #[test]
    fn starts_dot(input in gen::starts_with_dot()) {
        assert_matches!(RefString::try_from(input), Err(Error::StartsDot))
    }

    #[test]
    fn ends_dot(input in gen::ends_with_dot()) {
        assert_matches!(RefString::try_from(input), Err(Error::EndsDot))
    }

    #[test]
    fn control_char(input in gen::with_control_char()) {
        assert_matches!(RefString::try_from(input), Err(Error::Control))
    }

    #[test]
    fn space(input in gen::with_space()) {
        assert_matches!(RefString::try_from(input), Err(Error::Space))
    }

    #[test]
    fn consecutive_slashes(input in gen::with_consecutive_slashes()) {
        assert_matches!(RefString::try_from(input), Err(Error::Slash))
    }

    #[test]
    fn glob(input in gen::with_glob()) {
        assert_matches!(RefString::try_from(input), Err(Error::InvalidChar('*')))
    }

    #[test]
    fn invalid(input in gen::invalid()) {
        assert_matches!(RefString::try_from(input), Err(_))
    }

    #[test]
    fn roundtrip_components(input in gen::valid()) {
        assert_eq!(
            input.as_str(),
            RefStr::try_from_str(&input).unwrap().components().collect::<RefString>().as_str()
        )
    }

    #[test]
    fn json(input in gen::valid()) {
        let input = RefString::try_from(input).unwrap();
        roundtrip::json(input.clone());
        let qualified = refname!("refs/heads").and(input).qualified().unwrap().into_owned();
        roundtrip::json(qualified.clone());
        let namespaced = qualified.with_namespace(name::component!("foo"));
        roundtrip::json(namespaced);
    }

    #[test]
    fn json_value(input in gen::valid()) {
        let input = RefString::try_from(input).unwrap();
        roundtrip::json_value(input.clone());
        let qualified = refname!("refs/heads").and(input).qualified().unwrap().into_owned();
        roundtrip::json_value(qualified.clone());
        let namespaced = qualified.with_namespace(name::component!("foo"));
        roundtrip::json_value(namespaced);
    }

    #[test]
    fn cbor(input in gen::valid()) {
        roundtrip::cbor(RefString::try_from(input).unwrap())
    }
}
