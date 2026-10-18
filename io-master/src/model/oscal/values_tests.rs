//! Unit tests of `values.rs`.

use super::*;

#[test]
fn a_uuid_is_of_version_4() {
    let uuid = uuid_v4([0xff; 16]);

    assert_eq!(uuid, "ffffffff-ffff-4fff-bfff-ffffffffffff");
    assert_eq!(uuid_v4([0; 16]), "00000000-0000-4000-8000-000000000000");
}

#[test]
fn a_date_and_time_is_written_in_utc() {
    assert_eq!(date_time(0), "1970-01-01T00:00:00Z");
    assert_eq!(date_time(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(date_time(1_792_321_800), "2026-10-18T11:10:00Z");
}

#[test]
fn identifiers_become_tokens() {
    assert_eq!(token("FAU_GEN.1"), "FAU_GEN.1");
    assert_eq!(token("5.1-1"), "id_5.1-1");
    assert_eq!(token("Provision 5.1-1"), "Provision_5.1-1");
    assert_eq!(token(""), "id_");
}
