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
    // Isolate ~/.gitconfig so the include-sync never touches the real one.
    cmd.env("DUH_GITCONFIG", home.path().join("gitconfig"));
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
        .stdout(predicate::str::contains("package default"));
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
        .stdout(predicate::str::contains("default").and(predicate::str::contains("ls -al")));
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
        .stdout(predicate::str::contains("safe").and(predicate::str::contains("ssh-safe")));
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
        .stdout(predicate::str::contains("greet").and(predicate::str::contains("says hello")));
}

#[test]
fn inject_syncs_package_gitconfig_include() {
    let home = TempDir::new().unwrap();
    let gitconfig = home.path().join("dot.gitconfig");
    // Pre-existing user include must be preserved.
    std::fs::write(
        &gitconfig,
        "[include]\n\tpath = /home/me/.dotfiles/gitconfig\n",
    )
    .unwrap();

    // Package with a gitconfig file.
    duh(&home)
        .args(["add", "alias", "seed", "x"])
        .assert()
        .success();
    let pkg_gc = home.path().join("data/packages/default/gitconfig");
    std::fs::write(&pkg_gc, "[alias]\n\tco = checkout\n").unwrap();

    let run = || {
        duh(&home)
            .env("DUH_GITCONFIG", &gitconfig)
            .args(["inject", "--quiet"])
            .assert()
            .success();
    };
    run();
    let after = std::fs::read_to_string(&gitconfig).unwrap();
    assert!(
        after.contains("/home/me/.dotfiles/gitconfig"),
        "existing include preserved"
    );
    assert!(
        after.contains(&pkg_gc.display().to_string()),
        "package gitconfig included"
    );

    // Idempotent: second inject must not duplicate the line.
    run();
    let twice = std::fs::read_to_string(&gitconfig).unwrap();
    let count = twice.matches(&pkg_gc.display().to_string()).count();
    assert_eq!(count, 1, "include line must not be duplicated");
}

#[test]
fn add_git_alias_then_ls_git() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "git", "alias", "co", "checkout"])
        .assert()
        .success();
    duh(&home)
        .args(["add", "git", "alias", "st", "status"])
        .assert()
        .success();
    // Stored in the package gitconfig.
    let gc = std::fs::read_to_string(home.path().join("data/packages/default/gitconfig")).unwrap();
    assert!(gc.contains("co = checkout"), "gitconfig: {gc}");
    // ls git shows them.
    duh(&home)
        .args(["ls", "git"])
        .assert()
        .success()
        .stdout(predicate::str::contains("co").and(predicate::str::contains("checkout")));
    // rm removes one.
    duh(&home)
        .args(["rm", "git", "alias", "co"])
        .assert()
        .success();
    let gc2 = std::fs::read_to_string(home.path().join("data/packages/default/gitconfig")).unwrap();
    assert!(!gc2.contains("co = checkout"));
    assert!(gc2.contains("st = status"));
}

#[test]
fn completion_lists_packages_and_filters() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    // open → package names
    duh(&home)
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "duh", "open", ""])
        .assert()
        .success()
        .stdout(predicate::str::contains("default"));
    // ls → filters
    duh(&home)
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "duh", "ls", ""])
        .assert()
        .success()
        .stdout(predicate::str::contains("git").and(predicate::str::contains("alias")));
}

#[test]
fn pkg_rename_export_import_roundtrip() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["pkg", "create", "work"])
        .assert()
        .success();
    duh(&home).args(["use", "work"]).assert().success();
    duh(&home)
        .args(["add", "alias", "k", "kubectl"])
        .assert()
        .success();

    duh(&home)
        .args(["pkg", "rename", "work", "prod"])
        .assert()
        .success();
    assert!(home.path().join("data/packages/prod").exists());
    assert!(!home.path().join("data/packages/work").exists());

    let archive = home.path().join("prod.tgz");
    duh(&home)
        .args(["pkg", "export", "prod", "--out"])
        .arg(&archive)
        .assert()
        .success();
    assert!(archive.exists());

    duh(&home)
        .args(["pkg", "import"])
        .arg(&archive)
        .arg("prod2")
        .assert()
        .success();
    assert!(home.path().join("data/packages/prod2/db.toml").exists());
}

#[test]
fn man_renders_roff() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["man"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".TH").or(predicate::str::contains("duh")));
}

#[test]
fn edit_uses_editor() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls"])
        .assert()
        .success();
    duh(&home)
        .env("EDITOR", "true") // no-op editor
        .args(["edit"])
        .assert()
        .success()
        .stdout(predicate::str::contains("edited package"));
}

#[test]
fn schema_written_and_old_config_loads() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    let db = home.path().join("data/packages/default/db.toml");
    assert!(std::fs::read_to_string(&db).unwrap().contains("schema = 1"));
    // A db.toml without a schema field still loads (treated as v1).
    std::fs::write(&db, "[aliases]\nzz = \"echo z\"\n").unwrap();
    duh(&home)
        .args(["inject", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alias zz="));
}

#[test]
fn doctor_flags_missing_enabled_and_conflict() {
    let home = TempDir::new().unwrap();
    // Materialize the default package (enabled by default) so doctor is healthy.
    duh(&home)
        .args(["add", "alias", "seed", "x"])
        .assert()
        .success();
    // Two packages defining the same alias → conflict (warn).
    duh(&home).args(["pkg", "create", "a"]).assert().success();
    duh(&home).args(["pkg", "create", "b"]).assert().success();
    duh(&home).args(["use", "a"]).assert().success();
    duh(&home)
        .args(["add", "alias", "g", "git a"])
        .assert()
        .success();
    duh(&home).args(["use", "b"]).assert().success();
    duh(&home)
        .args(["add", "alias", "g", "git b"])
        .assert()
        .success();
    // ls shows the shadow marker.
    duh(&home)
        .args(["ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shadowed by"));
    // Healthy doctor (conflict is a warning) exits 0 and reports the conflict.
    duh(&home)
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wins"));
    // A missing enabled package is a hard error → exit 1.
    let prefs = home.path().join("config/prefs.toml");
    std::fs::write(
        &prefs,
        "[packages]\nenabled = [\"ghost\"]\ndefault = \"ghost\"\n",
    )
    .unwrap();
    duh(&home).args(["doctor"]).assert().failure();
}

#[test]
fn use_and_pkg_create() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    // bare use → current default
    duh(&home)
        .args(["use"])
        .assert()
        .success()
        .stdout(predicate::str::contains("default"));
    // create a new local package
    duh(&home)
        .args(["pkg", "create", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("created"));
    assert!(home.path().join("data/packages/work/db.toml").exists());
    // switch default to it
    duh(&home)
        .args(["use", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("now"));
    duh(&home)
        .args(["use"])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"));
    // add now targets the new default
    duh(&home)
        .args(["add", "alias", "k", "kubectl"])
        .assert()
        .success();
    assert!(
        std::fs::read_to_string(home.path().join("data/packages/work/db.toml"))
            .unwrap()
            .contains("kubectl")
    );
    // unknown package errors
    duh(&home)
        .args(["use", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no package"));
}

#[test]
fn machine_output_has_no_ansi() {
    // The eval'd paths must never carry color codes, even if forced on.
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    for args in [
        vec!["inject", "--quiet"],
        vec!["status", "--hook"],
        vec!["init", "--shell", "bash"],
    ] {
        let out = duh(&home).args(&args).output().unwrap();
        let s = String::from_utf8_lossy(&out.stdout);
        assert!(!s.contains('\u{1b}'), "{args:?} emitted an ANSI escape");
    }
}

#[test]
fn ls_json_is_valid_and_uncolored() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al", "--ssh-safe"])
        .assert()
        .success();
    let out = duh(&home).args(["ls", "--json"]).output().unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(!s.contains('\u{1b}'), "json must not be colored");
    let v: serde_json::Value = serde_json::from_str(&s).expect("valid json");
    let alias = &v["packages"][0]["aliases"][0];
    assert_eq!(alias["name"], "ll");
    assert_eq!(alias["value"], "ls -al");
    assert_eq!(alias["ssh_safe"], true);
}

#[test]
fn status_json_reports_counts() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    let out = duh(&home).args(["status", "--json"]).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid json");
    assert_eq!(v["aliases"], 1);
    assert_eq!(v["default"], "default");
}

#[test]
fn plain_is_ascii_only() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "ll", "ls -al"])
        .assert()
        .success();
    let out = duh(&home).args(["ls", "--plain"]).output().unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.is_ascii(), "--plain output must be ASCII-only: {s:?}");
    assert!(!s.contains('\u{1b}'));
}

/// Write a function file into the default package's functions dir.
fn write_fn(home: &TempDir, file: &str, body: &str) {
    let dir = home.path().join("data/packages/default/functions");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(file), body).unwrap();
}

#[test]
fn ls_fn_shows_script_function_tree() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "seed", "x"])
        .assert()
        .success();
    write_fn(
        &home,
        "git.sh",
        "#!/usr/bin/env bash\n# git status, short\ngs() { git status -s; }\n",
    );
    duh(&home).args(["ls", "fn"]).assert().success().stdout(
        predicate::str::contains("git.sh")
            .and(predicate::str::contains("gs"))
            .and(predicate::str::contains("git status, short")),
    );
}

#[test]
fn ls_fn_flag_shows_full_doc() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "seed", "x"])
        .assert()
        .success();
    write_fn(
        &home,
        "greet.sh",
        "# say hello\n# politely\ngreet() { echo hi; }\n",
    );
    // Describe view: script name, path, package, and the full doc block.
    duh(&home)
        .args(["ls", "--fn", "greet"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("greet.sh")
                .and(predicate::str::contains("functions/greet.sh"))
                .and(predicate::str::contains("package"))
                .and(predicate::str::contains("say hello"))
                .and(predicate::str::contains("politely")),
        );
    duh(&home)
        .args(["ls", "--fn", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no function named"));
}

#[test]
fn inject_strips_leading_shebang() {
    let home = TempDir::new().unwrap();
    duh(&home)
        .args(["add", "alias", "seed", "x"])
        .assert()
        .success();
    write_fn(&home, "f.sh", "#!/usr/bin/env bash\ngreet() { echo hi; }\n");
    let out = duh(&home).args(["inject", "--quiet"]).output().unwrap();
    let script = String::from_utf8(out.stdout).unwrap();
    assert!(
        !script.contains("#!/usr/bin/env bash"),
        "shebang must be stripped"
    );
    assert!(script.contains("greet() { echo hi; }"), "body must remain");
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
