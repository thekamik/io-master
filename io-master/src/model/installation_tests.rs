//! Unit tests of `model/installation.rs`.

use super::*;

#[test]
fn user_names_are_checked() {
    for name in ["anna", "Jan Kowalski", "ewaluator-2"] {
        assert!(check_user_name(name).is_ok(), "{name}");
    }

    let too_long = "n".repeat(USER_NAME_MAX + 1);

    for name in ["", " anna", "anna ", "an\nna", too_long.as_str()] {
        assert!(check_user_name(name).is_err(), "{name:?}");
    }
}

#[test]
fn roles_have_names_and_rights() {
    for role in [Role::Workstation, Role::Master] {
        assert_eq!(Role::from_name(role.name()), Some(role));
    }

    assert_eq!(Role::from_name("admin"), None);
    assert!(Role::Workstation.can_add_users());
    assert!(!Role::Master.can_add_users());
}
