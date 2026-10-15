//! Unit tests of `model.rs`.

use super::*;

#[test]
fn every_model_is_found_by_its_id() {
    for model in Model::ALL {
        assert_eq!(Model::from_id(model.id()), Some(model));
    }
}

#[test]
fn unknown_ids_are_not_models() {
    assert_eq!(Model::from_id("claude-opus-4-8"), None);
    assert_eq!(Model::from_id(""), None);
}

#[test]
fn ids_carry_no_date() {
    for model in Model::ALL {
        // A dated name ends with eight digits: "-20250514".
        let dated = model
            .id()
            .split('-')
            .any(|part| part.len() == 8 && part.bytes().all(|byte| byte.is_ascii_digit()));

        assert!(!dated, "{}", model.id());
    }
}

#[test]
fn opus_is_the_default() {
    assert_eq!(Model::DEFAULT.id(), "claude-opus-5-5");
}
