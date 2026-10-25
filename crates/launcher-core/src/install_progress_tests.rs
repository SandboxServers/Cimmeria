use super::seed::ArchiveCheck;

#[test]
fn archive_check_codes_are_the_two_closed_log_values() {
    assert_eq!(ArchiveCheck::Sha256("x").code(), "sha256");
    assert_eq!(
        ArchiveCheck::HashedWhileCopied.code(),
        "hashed_while_copied"
    );
}
