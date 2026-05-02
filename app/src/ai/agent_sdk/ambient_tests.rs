use super::*;

#[test]
fn hosted_ambient_commands_fail_closed() {
    let err = hosted_ambient_removed_error();
    assert!(err.to_string().contains("hosted ambient agent"));
}
