// cargo test --test bu_tests -- --nocapture

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Backup current directory"))
        .stdout(predicate::str::contains("-k")); // Verify -k is in the new help text
}

#[test]
fn test_backup_creation() {
    // 1. Setup a fake project environment
    let root = tempdir().unwrap();
    let project_path = root.path().join("my_project");
    let bak_path = root.path().join("bak");

    fs::create_dir(&project_path).unwrap();
    fs::write(project_path.join("file.txt"), "hello world").unwrap();

    // 2. Run 'bu save' inside that fake project
    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("save").current_dir(&project_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Saved backup: 000my_project.tar"));

    // 3. Verify the backup actually exists in the sibling folder
    assert!(bak_path.join("000my_project.tar").exists());
}

#[test]
fn test_invalid_command_fails() {
    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("nonexistent_command");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error: unknown command"));
}

#[test]
fn test_diff_functionality() {
    let root = tempdir().unwrap();
    let project_path = root.path().join("my_app");
    let _bak_path = root.path().join("bak"); // Created implicitly by bu

    fs::create_dir_all(&project_path).unwrap();

    let file_path = project_path.join("code.rs");
    fs::write(&file_path, "fn main() { println!(\"v1\"); }").unwrap();

    // 1. Initial backup
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&project_path)
        .assert()
        .success();

    // 2. Modify file
    fs::write(&file_path, "fn main() { println!(\"v2\"); }").unwrap();

    // 3. Run diff
    let mut cmd_diff = Command::cargo_bin("bu").unwrap();
    cmd_diff.arg("diff").current_dir(&project_path);

    cmd_diff
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "< fn main() { println!(\"v1\"); }",
        ))
        .stdout(predicate::str::contains(
            "> fn main() { println!(\"v2\"); }",
        ));
}

#[test]
fn test_diff_quiet_flag() {
    let root = tempdir().unwrap();
    let project_path = root.path().join("quiet_app");
    fs::create_dir_all(&project_path).unwrap();

    let file_path = project_path.join("code.rs");
    fs::write(&file_path, "fn main() { println!(\"v1\"); }").unwrap();

    // 1. Initial backup
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&project_path)
        .assert()
        .success();

    // 2. Modify file
    fs::write(&file_path, "fn main() { println!(\"v2\"); }").unwrap();

    // 3. Run diff with -q flag
    let mut cmd_diff = Command::cargo_bin("bu").unwrap();
    cmd_diff.arg("diff").arg("-q").current_dir(&project_path);

    // With -q, diff should show "Files ... differ" but not the actual diff lines
    cmd_diff
        .assert()
        .success()
        .stdout(predicate::str::contains("differ"))
        .stdout(predicate::str::contains("< fn main()").not())
        .stdout(predicate::str::contains("> fn main()").not());
}

#[test]
fn test_load_functionality() {
    let root = tempdir().unwrap();
    let project_path = root.path().join("restore_me");
    fs::create_dir_all(&project_path).unwrap();

    let file_path = project_path.join("data.txt");
    fs::write(&file_path, "original state").unwrap();

    // 1. Save state 000
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&project_path)
        .assert()
        .success();

    // 2. Change state
    fs::write(&file_path, "corrupted state").unwrap();

    // 3. Load state 000
    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("load").arg("0").current_dir(&project_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Restore complete"));

    // 4. Verify content is back to original
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "original state");
}

#[test]
fn test_find_functionality() {
    let root = tempdir().unwrap();
    let proj = root.path().join("search_proj");
    fs::create_dir_all(&proj).unwrap();

    // Index 000: contains "apple"
    fs::write(proj.join("fruit.txt"), "apple").unwrap();
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    // Index 001: contains "banana"
    fs::write(proj.join("fruit.txt"), "banana").unwrap();
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    // Search for "apple"
    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("find").arg("apple").current_dir(&proj);

    // Note: main.rs uses println!("[{:03}] found in {:?}", idx, path);
    // Debug format {:?} for Path adds quotes.
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "[000] found in \"search_proj/fruit.txt\"",
        ))
        .stdout(predicate::str::contains("[001]").not());
}

#[test]
fn test_trim_functionality() {
    let root = tempdir().unwrap();
    let proj = root.path().join("trim_proj");
    let bak = root.path().join("bak");
    fs::create_dir_all(&proj).unwrap();

    // Create 3 backups (000, 001, 002)
    for i in 0..3 {
        fs::write(proj.join("v.txt"), format!("version {}", i)).unwrap();
        Command::cargo_bin("bu")
            .unwrap()
            .arg("save")
            .current_dir(&proj)
            .assert()
            .success();
    }

    // Run trim
    Command::cargo_bin("bu")
        .unwrap()
        .arg("trim")
        .current_dir(&proj)
        .assert()
        .success();

    // 001 and 002 should be gone, latest (v2) should now be 000
    assert!(bak.join("000trim_proj.tar").exists());
    assert!(!bak.join("001trim_proj.tar").exists());
    assert!(!bak.join("002trim_proj.tar").exists());
}

#[test]
fn test_multi_index_diff() {
    let root = tempdir().unwrap();
    let proj = root.path().join("diff_proj");
    fs::create_dir_all(&proj).unwrap();

    fs::write(proj.join("note.txt"), "first").unwrap();
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    fs::write(proj.join("note.txt"), "second").unwrap();
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    // Diff 0 vs 1
    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("diff").arg("0").arg("1").current_dir(&proj);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("< first"))
        .stdout(predicate::str::contains("> second"));
}

#[test]
fn test_burc_excludes_functionality() {
    let root = tempdir().unwrap();

    // 1. Setup fake HOME directory structure
    let fake_home = root.path().join("fake_home");
    let burc_dir = fake_home.join(".burc");
    fs::create_dir_all(&burc_dir).unwrap();

    // 2. Create the excludes file and ignore "secret.txt"
    fs::write(burc_dir.join("excludes"), "secret.txt\n# comments should be ignored\n").unwrap();

    // 3. Setup the project
    let project_path = root.path().join("my_project");
    let bak_path = root.path().join("bak");
    fs::create_dir_all(&project_path).unwrap();

    fs::write(project_path.join("visible.txt"), "keep me").unwrap();
    fs::write(project_path.join("secret.txt"), "hide me").unwrap();

    // 4. Run 'bu save' with overridden HOME
    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("save")
        .current_dir(&project_path)
        .env("HOME", fake_home.to_str().unwrap()); // Override HOME

    cmd.assert().success();

    // 5. Verify the backup contains visible.txt but NOT secret.txt
    let tar_file = fs::File::open(bak_path.join("000my_project.tar")).unwrap();
    let mut archive = tar::Archive::new(tar_file);
    let mut found_secret = false;
    let mut found_visible = false;

    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap();
        if path.to_string_lossy().contains("secret.txt") {
            found_secret = true;
        }
        if path.to_string_lossy().contains("visible.txt") {
            found_visible = true;
        }
    }

    assert!(found_visible, "visible.txt should be in the backup");
    assert!(!found_secret, "secret.txt should have been excluded by ~/.burc/excludes");
}

#[test]
fn test_l_flag_alias() {
    let root = tempdir().unwrap();
    let proj = root.path().join("list_proj");
    fs::create_dir_all(&proj).unwrap();

    fs::write(proj.join("file.txt"), "data").unwrap();
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("-l").current_dir(&proj);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("000list_proj.tar"));
}

#[test]
fn test_m_flag_without_message_fails() {
    let root = tempdir().unwrap();
    let proj = root.path().join("error_proj");
    fs::create_dir_all(&proj).unwrap();

    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("-m").current_dir(&proj);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error: -m requires a message"));
}

#[test]
fn test_cat_functionality() {
    let root = tempdir().unwrap();
    let proj = root.path().join("cat_proj");
    fs::create_dir_all(&proj).unwrap();

    fs::write(proj.join("hello.txt"), "hello world").unwrap();
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("--cat").current_dir(&proj);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "# cat_proj/hello.txt ===================",
        ))
        .stdout(predicate::str::contains("hello world"));
}

#[test]
fn test_cat_c_alias() {
    let root = tempdir().unwrap();
    let proj = root.path().join("cat_alias");
    fs::create_dir_all(&proj).unwrap();

    fs::write(proj.join("note.txt"), "aliased content").unwrap();
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("-c").current_dir(&proj);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("aliased content"));
}

#[test]
fn test_cat_specific_index() {
    let root = tempdir().unwrap();
    let proj = root.path().join("cat_idx");
    fs::create_dir_all(&proj).unwrap();

    fs::write(proj.join("v.txt"), "first version").unwrap();
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    fs::write(proj.join("v.txt"), "second version").unwrap();
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    // --cat 0 should show the first version, not the second
    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("--cat").arg("0").current_dir(&proj);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("first version"))
        .stdout(predicate::str::contains("second version").not());
}

#[test]
fn test_cat_skips_binary() {
    let root = tempdir().unwrap();
    let proj = root.path().join("cat_bin");
    fs::create_dir_all(&proj).unwrap();

    // A file containing a NUL byte is treated as binary
    fs::write(proj.join("data.bin"), [0u8, 1, 2, 3, 4]).unwrap();
    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("bu").unwrap();
    cmd.arg("--cat").current_dir(&proj);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "# cat_bin/data.bin ===================",
        ))
        .stdout(predicate::str::contains("[binary file, 5 bytes, skipped]"));
}

#[test]
fn test_exclude_target_not_targets() {
    let root = tempdir().unwrap();
    let proj = root.path().join("seg_proj");
    let bak = root.path().join("bak");
    fs::create_dir_all(proj.join("target")).unwrap();
    fs::create_dir_all(proj.join("targets")).unwrap();

    fs::write(proj.join("target").join("junk.txt"), "build artifact").unwrap();
    fs::write(proj.join("targets").join("keep.txt"), "important").unwrap();

    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    let tar_file = fs::File::open(bak.join("000seg_proj.tar")).unwrap();
    let mut archive = tar::Archive::new(tar_file);
    let mut found_target = false;
    let mut found_targets = false;

    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().to_string();
        if path.contains("/target/junk.txt") {
            found_target = true;
        }
        if path.contains("/targets/keep.txt") {
            found_targets = true;
        }
    }

    assert!(!found_target, "target/ should be excluded");
    assert!(found_targets, "targets/ should NOT be excluded by a prefix match");
}

#[test]
fn test_rename_functionality() {
    let root = tempdir().unwrap();
    let proj = root.path().join("bu");
    let bak = root.path().join("bak");
    fs::create_dir_all(proj.join("src")).unwrap();
    fs::write(proj.join("src").join("main.rs"), "fn main() {}").unwrap();

    // Two backups: 000bu.tar, 001bu.tar
    Command::cargo_bin("bu").unwrap().arg("save").current_dir(&proj).assert().success();
    fs::write(proj.join("src").join("main.rs"), "fn main() { }").unwrap();
    Command::cargo_bin("bu").unwrap().arg("save").current_dir(&proj).assert().success();

    Command::cargo_bin("bu")
        .unwrap()
        .arg("--rename")
        .arg("bu")
        .arg("bu2")
        .current_dir(&proj)
        .assert()
        .success()
        .stdout(predicate::str::contains("000bu.tar -> 000bu2.tar"))
        .stdout(predicate::str::contains("001bu.tar -> 001bu2.tar"));

    // New names exist, old names gone
    assert!(bak.join("000bu2.tar").exists());
    assert!(bak.join("001bu2.tar").exists());
    assert!(!bak.join("000bu.tar").exists());
    assert!(!bak.join("001bu.tar").exists());

    // Inner paths rewritten: every entry starts with bu2/, none with bu/
    let tar_file = fs::File::open(bak.join("000bu2.tar")).unwrap();
    let mut archive = tar::Archive::new(tar_file);
    let mut saw_main = false;
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().to_string();
        assert!(
            path == "bu2" || path.starts_with("bu2/"),
            "unexpected inner path: {}",
            path
        );
        if path == "bu2/src/main.rs" {
            saw_main = true;
        }
    }
    assert!(saw_main, "expected rewritten bu2/src/main.rs inside the archive");
}

#[test]
fn test_rename_moves_message_file() {
    let root = tempdir().unwrap();
    let proj = root.path().join("msgproj");
    let bak = root.path().join("bak");
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("f.txt"), "data").unwrap();

    Command::cargo_bin("bu")
        .unwrap()
        .arg("-m")
        .arg("a note")
        .current_dir(&proj)
        .assert()
        .success();

    Command::cargo_bin("bu")
        .unwrap()
        .arg("--rename")
        .arg("msgproj")
        .arg("renamed")
        .current_dir(&proj)
        .assert()
        .success();

    assert!(!bak.join("msgproj.txt").exists());
    let msg = fs::read_to_string(bak.join("renamed.txt")).unwrap();
    assert!(msg.contains("000: a note"));
}

#[test]
fn test_rename_fails_if_new_exists() {
    let root = tempdir().unwrap();
    let proj = root.path().join("orig");
    let bak = root.path().join("bak");
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("f.txt"), "data").unwrap();

    Command::cargo_bin("bu").unwrap().arg("save").current_dir(&proj).assert().success();

    // Pre-create a colliding 000dest.tar
    fs::write(bak.join("000dest.tar"), b"not a real tar").unwrap();

    Command::cargo_bin("bu")
        .unwrap()
        .arg("--rename")
        .arg("orig")
        .arg("dest")
        .current_dir(&proj)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    // Original archive is untouched
    assert!(bak.join("000orig.tar").exists());
}

#[test]
fn test_rename_no_backups_fails() {
    let root = tempdir().unwrap();
    let proj = root.path().join("empty");
    fs::create_dir_all(&proj).unwrap();
    // Create the bak dir so it exists but has no matching archives
    fs::create_dir_all(root.path().join("bak")).unwrap();

    Command::cargo_bin("bu")
        .unwrap()
        .arg("--rename")
        .arg("nothere")
        .arg("whatever")
        .current_dir(&proj)
        .assert()
        .failure()
        .stderr(predicate::str::contains("no backups found"));
}

fn status_alias_shows_only_names(alias: &str) {
    let root = tempdir().unwrap();
    let proj = root.path().join("status_proj");
    fs::create_dir_all(&proj).unwrap();

    let file_path = proj.join("code.rs");
    fs::write(&file_path, "fn main() { println!(\"v1\"); }").unwrap();

    Command::cargo_bin("bu")
        .unwrap()
        .arg("save")
        .current_dir(&proj)
        .assert()
        .success();

    fs::write(&file_path, "fn main() { println!(\"v2\"); }").unwrap();

    Command::cargo_bin("bu")
        .unwrap()
        .arg(alias)
        .current_dir(&proj)
        .assert()
        .success()
        .stdout(predicate::str::contains("differ"))
        .stdout(predicate::str::contains("< fn main()").not())
        .stdout(predicate::str::contains("> fn main()").not());
}

#[test]
fn test_status_long_flag() {
    status_alias_shows_only_names("--status");
}

#[test]
fn test_status_short_flag() {
    status_alias_shows_only_names("-s");
}

#[test]
fn test_status_bare_word() {
    status_alias_shows_only_names("s");
}

#[test]
fn test_diff_file_list() {
    let root = tempdir().unwrap();
    let proj = root.path().join("dl_proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("a.txt"), "a1").unwrap();
    fs::write(proj.join("b.txt"), "b1").unwrap();
    fs::write(proj.join("c.txt"), "c1").unwrap();

    Command::cargo_bin("bu").unwrap().arg("save").current_dir(&proj).assert().success();

    fs::write(proj.join("a.txt"), "a2").unwrap();
    fs::write(proj.join("b.txt"), "b2").unwrap();
    fs::write(proj.join("c.txt"), "c2").unwrap();

    // Only a.txt and b.txt should be diffed, not c.txt
    Command::cargo_bin("bu")
        .unwrap()
        .arg("diff")
        .arg("a.txt, b.txt")
        .current_dir(&proj)
        .assert()
        .success()
        .stdout(predicate::str::contains("< a1"))
        .stdout(predicate::str::contains("> a2"))
        .stdout(predicate::str::contains("< b1"))
        .stdout(predicate::str::contains("< c1").not())
        .stdout(predicate::str::contains("> c2").not());
}

#[test]
fn test_diff_file_list_quiet() {
    let root = tempdir().unwrap();
    let proj = root.path().join("dlq_proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("a.txt"), "a1").unwrap();
    fs::write(proj.join("b.txt"), "b1").unwrap();

    Command::cargo_bin("bu").unwrap().arg("save").current_dir(&proj).assert().success();

    fs::write(proj.join("a.txt"), "a2").unwrap();
    fs::write(proj.join("b.txt"), "b2").unwrap();

    // Only a.txt is listed; -q reports it differs and does not mention b.txt or line content
    Command::cargo_bin("bu")
        .unwrap()
        .arg("diff")
        .arg("a.txt")
        .arg("-q")
        .current_dir(&proj)
        .assert()
        .success()
        .stdout(predicate::str::contains("a.txt"))
        .stdout(predicate::str::contains("differ"))
        .stdout(predicate::str::contains("b.txt").not())
        .stdout(predicate::str::contains("< a1").not());
}

#[test]
fn test_archive_diff_file_list() {
    let root = tempdir().unwrap();
    let proj = root.path().join("adl_proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("a.txt"), "a1").unwrap();
    fs::write(proj.join("b.txt"), "b1").unwrap();

    Command::cargo_bin("bu").unwrap().arg("save").current_dir(&proj).assert().success();

    fs::write(proj.join("a.txt"), "a2").unwrap();
    fs::write(proj.join("b.txt"), "b2").unwrap();
    Command::cargo_bin("bu").unwrap().arg("save").current_dir(&proj).assert().success();

    // Archive 0 vs 1, limited to a.txt
    Command::cargo_bin("bu")
        .unwrap()
        .arg("diff")
        .arg("0")
        .arg("1")
        .arg("a.txt")
        .current_dir(&proj)
        .assert()
        .success()
        .stdout(predicate::str::contains("< a1"))
        .stdout(predicate::str::contains("> a2"))
        .stdout(predicate::str::contains("b1").not())
        .stdout(predicate::str::contains("b2").not());
}

