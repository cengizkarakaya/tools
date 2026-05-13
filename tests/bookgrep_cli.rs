use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn bookgrep_help_is_available() {
    let mut cmd = Command::cargo_bin("bookgrep").expect("bookgrep binary");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(contains("Search PDF and EPUB books"));
}
