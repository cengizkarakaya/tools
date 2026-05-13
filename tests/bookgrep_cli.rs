use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn bookgrep_help_is_available() {
    let mut cmd = Command::cargo_bin("bookgrep").expect("bookgrep binary");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Search PDF and EPUB books"));
}

#[test]
fn bookgrep_search_help_is_readable_and_colored() {
    let mut cmd = Command::cargo_bin("bookgrep").expect("bookgrep binary");
    cmd.args(["search", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\u{1b}[1m")
                .and(predicate::str::contains("Examples:"))
                .and(predicate::str::contains(
                    "Search books under a local folder",
                ))
                .and(predicate::str::contains(
                    "Limit search to a format; can be repeated",
                )),
        )
        .stdout(
            predicate::str::contains("--matches")
                .and(predicate::str::contains("Show every matched passage")),
        );
}
