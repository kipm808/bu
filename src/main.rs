use chrono::{DateTime, Local};
use std::collections::HashMap;
use std::env;
use std::fs::{self};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use tar::{Archive, Builder};

fn main() {
    let args: Vec<String> = env::args().collect();
    let current_dir = env::current_dir().expect("failed to get current directory");
    let tgt_dir_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .expect("failed to get directory name")
        .to_string();

    let bak_dir = current_dir.parent().unwrap().join("bak");
    let msg_file = bak_dir.join(format!("{}.txt", tgt_dir_name));

    let excludes = get_excludes();

    if args.len() > 1 {
        let first_arg = args[1].as_str();
        if first_arg == "-h" || first_arg == "--help" || first_arg == "help" {
            print_usage(&args[0]);
            exit(0);
        }
    }

    match args.get(1).map(|s| s.as_str()) {
        None | Some("save") | Some("-m") => {
            if args.get(1) == Some(&"-m".to_string()) && args.len() == 2 {
                eprintln!("error: -m requires a message");
                exit(1);
            }
            if !bak_dir.exists() {
                fs::create_dir_all(&bak_dir).expect("failed to create bak directory");
            }
            let message = extract_message(&args);
            handle_default_bu(&bak_dir, &tgt_dir_name, &current_dir, &msg_file, message, &excludes);
        }
        Some("ls") | Some("l") | Some("-l") => handle_list(&bak_dir, &tgt_dir_name, &msg_file),
        Some("trim") => handle_purge(&bak_dir, &tgt_dir_name, &msg_file),
        Some("load") => {
            let provided_idx = args.get(2).and_then(|s| s.parse::<u32>().ok());
            let target_idx = provided_idx.or_else(|| get_latest_idx(&bak_dir, &tgt_dir_name));
            if let Some(idx) = target_idx {
                handle_restore(idx, &bak_dir, &tgt_dir_name, &current_dir);
            } else {
                eprintln!("error: no backups found to load");
                exit(1);
            }
        }
        Some("find") => {
            if let Some(pattern) = args.get(2) {
                let file_filter = args.get(3).cloned();
                handle_search(&bak_dir, &tgt_dir_name, pattern, file_filter);
            } else {
                eprintln!("usage: bu find <pattern> [file_filter]");
            }
        }
        Some("diff") => {
            let idx1 = args.get(2).and_then(|s| s.parse::<u32>().ok());
            let idx2 = args.get(3).and_then(|s| s.parse::<u32>().ok());

            match (idx1, idx2) {
                (Some(i1), Some(i2)) => {
                    let (keep, file) = parse_extra_args(&args, 4);
                    run_archive_diff(i1, i2, &bak_dir, &tgt_dir_name, keep, file);
                }
                (Some(i1), None) => {
                    let (keep, file) = parse_extra_args(&args, 3);
                    run_diff(i1, &bak_dir, &tgt_dir_name, &current_dir, keep, file);
                }
                _ => {
                    let (keep, file) = parse_extra_args(&args, 2);
                    handle_diff(&bak_dir, &tgt_dir_name, &current_dir, keep, file);
                }
            }
        }
        Some(other) => {
            eprintln!("error: unknown command '{}'", other);
            print_usage(&args[0]);
            exit(1);
        }
    }
}

fn get_excludes() -> Vec<String> {
    let excludes = vec![
        "bak".to_string(),
        "tmp".to_string(),
        "objs".to_string(),
        "build".to_string(),
        ".git".to_string(),
        "__pycache__".to_string(),
        "target".to_string(),
    ];

    if let Some(home_path) = env::var_os("HOME").map(PathBuf::from) {
        let rc_path = home_path.join(".burc").join("excludes");
        if let Ok(file) = fs::File::open(rc_path) {
            let reader = BufReader::new(file);
            let custom_excludes: Vec<String> = reader
                .lines()
                .map_while(Result::ok)
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();
            
            if !custom_excludes.is_empty() {
                return custom_excludes;
            }
        }
    }
    excludes
}

fn extract_message(args: &[String]) -> Option<String> {
    if let Some(pos) = args.iter().position(|x| x == "-m") {
        let msg_parts: Vec<String> = args.iter().skip(pos + 1).cloned().collect();
        if msg_parts.is_empty() {
            return None;
        }
        let full_msg = msg_parts.join(" ").trim().to_string();
        if full_msg.is_empty() { None } else { Some(full_msg) }
    } else {
        None
    }
}

fn parse_extra_args(args: &[String], start_idx: usize) -> (bool, Option<String>) {
    let mut keep = false;
    let mut file = None;
    for arg in args.iter().skip(start_idx) {
        if arg == "-k" || arg == "keep" || arg == "--keep" {
            keep = true;
        } else if file.is_none() && !arg.starts_with('-') {
            file = Some(arg.clone());
        }
    }
    (keep, file)
}

fn print_usage(bin_name: &str) {
    let name = Path::new(bin_name).file_name().unwrap().to_str().unwrap();
    println!("Usage: {} <command> [args]", name);
    println!("\nCommands:");
    println!("  save [-m MSG]       Backup current directory to ../bak/ (default)");
    println!("  -m MSG...           Shorthand for save with multi-word message");
    println!("  ls, l, -l           List backups and messages");
    println!("  load [idx]          Restore backup (defaults to latest if idx omitted)");
    println!("  find <pat> [file]   Search for pattern in historical files");
    println!("  diff [i1] [i2] [-k] Diff latest vs current, or archive vs archive");
    println!("                      (-k: keep extracted files in /tmp/)");
    println!("  trim                Keep only latest backup and reset to 000");
}

fn handle_default_bu(
    bak_dir: &Path,
    tgt_name: &str,
    current_dir: &Path,
    msg_file: &Path,
    message: Option<String>,
    excludes: &[String],
) {
    let idx = get_latest_idx(bak_dir, tgt_name).map_or(0, |i| i + 1);
    let tar_path = bak_dir.join(format!("{:03}{}.tar", idx, tgt_name));

    let file = fs::File::create(&tar_path).expect("failed to create tar file");
    let mut enc = Builder::new(file);

    let parent = current_dir.parent().unwrap();
    env::set_current_dir(parent).unwrap();

    for entry in walkdir::WalkDir::new(tgt_name) {
        let entry = entry.unwrap();
        let path = entry.path();
        if !should_exclude(path, excludes) {
            if path.is_dir() {
                enc.append_dir(path, path).ok();
            } else if let Ok(mut f) = fs::File::open(path) {
                enc.append_file(path, &mut f).ok();
            }
        }
    }

    if let Some(m) = message {
        let mut lines = Vec::new();
        if msg_file.exists() {
            let f = fs::File::open(msg_file).unwrap();
            let reader = BufReader::new(f);
            let prefix = format!("{:03}:", idx);
            for line in reader.lines().map_while(Result::ok) {
                if !line.starts_with(&prefix) {
                    lines.push(line);
                }
            }
        }
        lines.push(format!("{:03}: {}", idx, m));
        fs::write(msg_file, lines.join("\n") + "\n").ok();
    }

    println!("Saved backup: {:03}{}.tar", idx, tgt_name);
}

fn handle_list(bak_dir: &Path, tgt_name: &str, msg_file: &Path) {
    if !bak_dir.exists() { return; }
    let mut messages = HashMap::new();
    if let Ok(file) = fs::File::open(msg_file) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if let Some((idx_part, msg_part)) = line.split_once(": ") {
                messages.insert(idx_part.to_string(), msg_part.to_string());
            }
        }
    }

    let mut idx = 0;
    loop {
        let tar_path = bak_dir.join(format!("{:03}{}.tar", idx, tgt_name));
        if !tar_path.exists() { break; }
        if let Ok(meta) = fs::metadata(&tar_path) {
            let datetime: DateTime<Local> = meta.modified().unwrap().into();
            let msg = messages.get(&format!("{:03}", idx)).cloned().unwrap_or_default();
            println!("{:>5} {} {:03}{}.tar  {}", 
                format_size(meta.len()), 
                datetime.format("%b %d %H:%M"), 
                idx, 
                tgt_name, 
                msg
            );
        }
        idx += 1;
    }
}

fn handle_purge(bak_dir: &Path, tgt_name: &str, msg_file: &Path) {
    let latest_idx = match get_latest_idx(bak_dir, tgt_name) {
        Some(idx) => idx,
        None => return,
    };

    let mut latest_msg = None;
    if let Ok(file) = fs::File::open(msg_file) {
        let reader = BufReader::new(file);
        let prefix = format!("{:03}: ", latest_idx);
        for line in reader.lines().map_while(Result::ok) {
            if line.starts_with(&prefix) {
                latest_msg = Some(line[5..].to_string());
                break;
            }
        }
    }

    for i in 0..latest_idx {
        let path = bak_dir.join(format!("{:03}{}.tar", i, tgt_name));
        let _ = fs::remove_file(&path);
    }

    let old_path = bak_dir.join(format!("{:03}{}.tar", latest_idx, tgt_name));
    let new_path = bak_dir.join(format!("000{}.tar", tgt_name));
    let _ = fs::rename(&old_path, &new_path);

    if let Some(m) = latest_msg {
        fs::write(msg_file, format!("000: {}\n", m)).ok();
    } else {
        let _ = fs::remove_file(msg_file);
    }
    println!("Trimmed history. Latest backup is now 000.");
}

fn handle_restore(idx: u32, bak_dir: &Path, tgt_name: &str, current_dir: &Path) {
    let tar_path = bak_dir.join(format!("{:03}{}.tar", idx, tgt_name));
    if !tar_path.exists() {
        eprintln!("error: backup {:03} not found", idx);
        exit(1);
    }
    let excludes = ["bak", ".git", "target"];
    for entry in fs::read_dir(current_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if !excludes.contains(&name) {
            if path.is_dir() {
                let _ = fs::remove_dir_all(&path);
            } else {
                let _ = fs::remove_file(&path);
            }
        }
    }
    let mut archive = Archive::new(fs::File::open(&tar_path).unwrap());
    archive.unpack(current_dir.parent().unwrap()).expect("failed to restore");
    println!("Restore complete.");
}

fn handle_search(bak_dir: &Path, tgt_name: &str, pattern: &str, file_filter: Option<String>) {
    if !bak_dir.exists() { return; }
    let mut idx = 0;
    loop {
        let tar_path = bak_dir.join(format!("{:03}{}.tar", idx, tgt_name));
        if !tar_path.exists() { break; }
        let mut archive = Archive::new(fs::File::open(&tar_path).unwrap());
        if let Ok(entries) = archive.entries() {
            for mut f in entries.flatten() {
                let path = f.path().unwrap().to_path_buf();
                if f.header().entry_type().is_dir() { continue; }
                if let Some(ref filter) = file_filter {
                    if !path.to_string_lossy().contains(filter) { continue; }
                }
                let mut content = String::new();
                if f.read_to_string(&mut content).is_ok() && content.contains(pattern) {
                    println!("[{:03}] found in {:?}", idx, path);
                }
            }
        }
        idx += 1;
    }
}

fn handle_diff(bak_dir: &Path, tgt_name: &str, cur: &Path, keep: bool, file: Option<String>) {
    if let Some(idx) = get_latest_idx(bak_dir, tgt_name) {
        run_diff(idx, bak_dir, tgt_name, cur, keep, file);
    }
}

fn run_diff(idx: u32, bak_dir: &Path, tgt_name: &str, current_dir: &Path, keep: bool, file: Option<String>) {
    let tar_path = bak_dir.join(format!("{:03}{}.tar", idx, tgt_name));
    let tmp_parent = tempfile::tempdir().unwrap();
    let extracted_dir = tmp_parent.path().join(tgt_name);
    Archive::new(fs::File::open(&tar_path).unwrap()).unpack(tmp_parent.path()).unwrap();
    
    let mut diff_cmd = Command::new("diff");
    if let Some(filename) = file {
        diff_cmd.arg(extracted_dir.join(&filename)).arg(current_dir.join(&filename));
    } else {
        diff_cmd.arg("-r").arg(&extracted_dir).arg(current_dir);
    }
    let _ = diff_cmd.status();
    
    if keep {
        let dest = PathBuf::from("/tmp").join(tgt_name);
        if dest.exists() { let _ = fs::remove_dir_all(&dest); }
        let _ = fs::rename(&extracted_dir, &dest);
        println!("Preserved: /tmp/{}", tgt_name);
    }
}

fn run_archive_diff(idx1: u32, idx2: u32, bak_dir: &Path, tgt_name: &str, keep: bool, file: Option<String>) {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    Archive::new(fs::File::open(bak_dir.join(format!("{:03}{}.tar", idx1, tgt_name))).unwrap()).unpack(tmp1.path()).unwrap();
    Archive::new(fs::File::open(bak_dir.join(format!("{:03}{}.tar", idx2, tgt_name))).unwrap()).unpack(tmp2.path()).unwrap();
    
    let dir1 = tmp1.path().join(tgt_name);
    let dir2 = tmp2.path().join(tgt_name);
    let mut diff_cmd = Command::new("diff");
    if let Some(filename) = file {
        diff_cmd.arg(dir1.join(&filename)).arg(dir2.join(&filename));
    } else {
        diff_cmd.arg("-r").arg(&dir1).arg(&dir2);
    }
    let _ = diff_cmd.status();

    if keep {
        let dest1 = PathBuf::from("/tmp").join(format!("{}_{:03}", tgt_name, idx1));
        let dest2 = PathBuf::from("/tmp").join(format!("{}_{:03}", tgt_name, idx2));
        let _ = fs::rename(&dir1, &dest1);
        let _ = fs::rename(&dir2, &dest2);
        println!("Preserved: /tmp/{}_{:03} and /tmp/{}_{:03}", tgt_name, idx1, tgt_name, idx2);
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1073741824 { format!("{:.1}G", bytes as f64 / 1073741824.0) }
    else if bytes >= 1048576 { format!("{:.1}M", bytes as f64 / 1048576.0) }
    else if bytes >= 1024 { format!("{:.1}K", bytes as f64 / 1024.0) }
    else { format!("{}B", bytes) }
}

fn should_exclude(path: &Path, excludes: &[String]) -> bool {
    let path_str = path.to_str().unwrap_or("");
    excludes.iter().any(|ex| path_str.contains(ex))
        || path.extension().is_some_and(|ext| ext == "sif")
}

fn get_latest_idx(bak_dir: &Path, tgt_name: &str) -> Option<u32> {
    let mut idx = 0;
    while bak_dir.join(format!("{:03}{}.tar", idx, tgt_name)).exists() {
        idx += 1;
    }
    if idx == 0 { None } else { Some(idx - 1) }
}
