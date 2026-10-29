use assert_cmd::Command;

fn polykov() -> Command {
    Command::cargo_bin("polykov").unwrap()
}

fn stdout_of(args: &[&str]) -> String {
    let out = polykov().args(args).assert().success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn version_prints_crate_version() {
    let stdout = stdout_of(&["--version"]);
    assert_eq!(
        stdout.trim(),
        format!("polykov {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_lists_no_subcommands() {
    let stdout = stdout_of(&["--help"]);
    assert!(
        !stdout.contains("Commands:"),
        "unexpected subcommands:\n{stdout}"
    );
}

#[test]
fn unknown_subcommand_fails() {
    polykov().arg("status").assert().failure();
}
