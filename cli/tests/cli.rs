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
fn help_lists_implemented_subcommands_and_global_flags() {
    let stdout = stdout_of(&["--help"]);
    let commands: Vec<&str> = stdout
        .split("Commands:")
        .nth(1)
        .unwrap()
        .split("Options:")
        .next()
        .unwrap()
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    assert_eq!(
        commands,
        [
            "login",
            "logout",
            "status",
            "new",
            "show",
            "ls",
            "edit",
            "rm",
            "pin",
            "unpin",
            "archive",
            "unarchive",
            "link",
            "comment",
            "trash",
            "attach",
            "help"
        ]
    );
    assert!(stdout.contains("--server <URL>"));
    assert!(stdout.contains("--json"));
}

#[test]
fn unknown_subcommand_fails() {
    polykov().arg("frobnicate").assert().failure();
}
