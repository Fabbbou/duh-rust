//! End-to-end CLI tests with isolated data/config/cache dirs.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Build a `duh` command bound to a fresh temp environment.
fn duh(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("duh").unwrap();
    cmd.env("DUH_DATA_DIR", home.path().join("data"));
    cmd.env("DUH_CONFIG_DIR", home.path().join("config"));
    cmd.env("DUH_CACHE_DIR", home.path().join("cache"));
    cmd
}

#[test]
fn add_alias_then_inject_shows_it() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    duh(&home)
        .args(["inject", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alias ll='ls -al'"));
}

#[test]
fn add_export_then_inject_shows_it() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "export", "EDITOR", "nvim"])
        .assert()
        .success();
    duh(&home)
        .args(["inject", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("export EDITOR='nvim'"));
}

#[test]
fn malicious_value_is_neutralized() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "x", "$(rm -rf /)"])
        .assert()
        .success();
    duh(&home)
        .args(["inject", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alias x='$(rm -rf /)'"));
}

#[test]
fn invalid_alias_name_rejected() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "a;b", "x"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid alias name"));
}

#[test]
fn rm_alias_removes_it() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "gone", "x"])
        .assert()
        .success();
    duh(&home).args(["rm", "alias", "gone"]).assert().success();
    duh(&home)
        .args(["inject", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("gone").not());
}

#[test]
fn ls_lists_added_entries() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    duh(&home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ll").and(predicate::str::contains("ls -al")));
}

#[test]
fn status_hook_prints_reload_when_stale() {
    let home = TempDir::new().unwrap();
    // No cache yet → hook should emit a reload command.
    duh(&home)
        .args(["status", "--hook"])
        .assert()
        .success()
        .stdout(predicate::str::contains("duh inject --quiet"));
}

#[test]
fn status_hook_silent_when_in_sync() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    duh(&home).args(["inject", "--quiet"]).assert().success();
    // Immediately after inject, nothing changed → hook is silent.
    duh(&home)
        .args(["status", "--hook"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn pkg_rm_rejects_path_traversal() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["pkg", "rm", "../../etc"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid package name"));
}

#[test]
fn control_char_value_rejected_on_inject() {
    // Simulate a malicious cloned package: a db.toml whose export value carries
    // a control char (BEL). It must be refused at generation time. (NUL in argv
    // is already blocked by the OS, so the file is the real vector.)
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "seed", "x"])
        .assert()
        .success(); // bootstrap dirs
    let db = home.path().join("data/packages/default/db.toml");
    std::fs::write(&db, "[exports]\nFOO = \"a\\u0007b\"\n").unwrap();
    duh(&home)
        .args(["inject", "--quiet"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("control character"));
}

#[test]
fn inject_script_is_owner_only() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let home = TempDir::new().unwrap();
        duh(&home)
            .args(["add", "alias", "ll", "ls -al"])
            .assert()
            .success();
        duh(&home).args(["inject", "--quiet"]).assert().success();
        let script = home.path().join("cache/inject.sh");
        let mode = std::fs::metadata(&script).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "inject.sh must be 0600, got {mode:o}");
    }
}

#[test]
fn init_emits_shell_snippet() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["init", "--shell", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add-zsh-hook precmd"));
}

#[test]
fn uninstall_yes_keeps_packages_removes_cache() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    duh(&home).args(["inject", "--quiet"]).assert().success();
    assert!(home.path().join("cache").exists());

    duh(&home)
        .env("DUH_KEEP_BINARY", "1")
        .args(["uninstall", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kept packages"));

    assert!(
        !home.path().join("cache").exists(),
        "cache should be removed"
    );
    assert!(home.path().join("data").exists(), "packages should be kept");
}

#[test]
fn uninstall_purge_deletes_everything() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    duh(&home).args(["inject", "--quiet"]).assert().success();

    duh(&home)
        .env("DUH_KEEP_BINARY", "1")
        .args(["uninstall", "--purge"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed packages"));

    assert!(
        !home.path().join("data").exists(),
        "packages should be deleted"
    );
    assert!(
        !home.path().join("config").exists(),
        "config should be deleted"
    );
    assert!(
        !home.path().join("cache").exists(),
        "cache should be deleted"
    );
}

#[test]
fn default_package_not_created_until_used() {
    let home = TempDir::new().unwrap();
    let default_dir = home.path().join("data/packages/default");

    // Commands that only bootstrap must NOT materialize the default package.
    duh(&home).args(["status"]).assert().success();
    duh(&home).args(["where"]).assert().success();
    duh(&home).args(["ls"]).assert().success();
    assert!(
        !default_dir.exists(),
        "bootstrap must not create the default package dir"
    );

    // It appears only once something is actually written to it.
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    assert!(
        default_dir.exists(),
        "default package should be created lazily on first add"
    );
}

#[test]
fn add_echoes_target_package() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success()
        .stdout(predicate::str::contains("package \"default\""));
}

#[test]
fn inject_includes_shell_helpers() {
    let home = TempDir::new().unwrap();
    let out = duh(&home).args(["inject", "--quiet"]).output().unwrap();
    let script = String::from_utf8(out.stdout).unwrap();
    assert!(script.contains("duh-reload()"), "missing duh-reload helper");
    assert!(script.contains("duh-cd()"), "missing duh-cd helper");
    assert!(
        script.contains("duh-cd-config()"),
        "missing duh-cd-config helper"
    );
}

#[test]
fn where_lists_paths() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["where"])
        .assert()
        .success()
        .stdout(predicate::str::contains("packages").and(predicate::str::contains("config dir")));
}

#[test]
fn ls_package_filter_and_unknown() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    duh(&home)
        .args(["ls", "--package", "default"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[default]"));
    duh(&home)
        .args(["ls", "--package", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no package"));
}

#[test]
fn ssh_safe_only_filters_local_inject() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "safe", "ls", "--ssh-safe"])
        .assert()
        .success();
    duh(&home)
        .args(["add", "alias", "secret", "echo s"])
        .assert()
        .success();
    // Local inject shows both; ls marks the safe one.
    duh(&home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("safe").and(predicate::str::contains("[ssh-safe]")));
    duh(&home)
        .args(["inject", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alias secret="));
}

#[test]
fn reload_command_removed() {
    let home = TempDir::new().unwrap();
    duh(&home).args(["reload"]).assert().failure();
}

#[test]
fn fn_doc_comment_shown_in_ls() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "seed", "x"])
        .assert()
        .success();
    let dir = home.path().join("data/packages/default/functions");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("greet.sh"), "# says hello\ngreet() { echo hi; }\n").unwrap();
    duh(&home)
        .args(["ls", "fn"])
        .assert()
        .success()
        .stdout(predicate::str::contains("greet — says hello"));
}

#[test]
fn generated_script_is_valid_bash() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    duh(&home)
        .args(["add", "export", "FOO", "a'b\"c $x"])
        .assert()
        .success();
    let out = duh(&home).args(["inject", "--quiet"]).output().unwrap();
    let script = String::from_utf8(out.stdout).unwrap();
    // The local inject targets bash/zsh (where `duh init` wires it); the
    // injected helpers use bash-valid function names. Validate with `bash -n`.
    let mut sh = Command::new("bash");
    sh.args(["-n", "-c", &script]);
    sh.assert().success();
}
