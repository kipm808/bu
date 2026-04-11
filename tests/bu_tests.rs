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

