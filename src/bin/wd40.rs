// WD-40 CLI: scan and clean dev build artifacts from the terminal.
// Usage: wd40 [scan|clean|clean-old|help] [options]
use rust_cleaner::cleaner::{clean_all, clean_old};
use rust_cleaner::config::Config;
use rust_cleaner::scanner::{human_size, scan_discover, scan_sizes, ArtifactGroup, TargetDir};
use std::time::Duration;

const SECONDS_PER_DAY: u64 = 86_400;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("scan");

    match cmd {
        "scan" | "s" => cmd_scan(&args[2..]),
        "clean" | "c" => cmd_clean(&args[2..]),
        "clean-old" | "co" => cmd_clean_old(&args[2..]),
        "help" | "-h" | "--help" => print_help(),
        other => {
            eprintln!("Unknown command: {}", other);
            print_help();
            std::process::exit(1);
        }
    }
}

fn load_targets(args: &[String]) -> Vec<TargetDir> {
    let config = Config::load();
    eprint!("Scanning...");
    let mut targets = scan_discover(&config);
    scan_sizes(&mut targets);
    eprintln!(" found {} targets", targets.len());

    if let Some(group) = parse_group_flag(args) {
        targets.retain(|t| t.kind.group() == group);
    }
    targets
}

fn cmd_scan(args: &[String]) {
    let targets = load_targets(args);
    print_table(&targets);
}

fn cmd_clean(args: &[String]) {
    let dry_run = has_flag(args, &["--dry-run", "-n"]);
    let targets = load_targets(args);

    if targets.is_empty() {
        println!("Nothing to clean.");
        return;
    }

    print_table(&targets);
    let total: u64 = targets.iter().map(|t| t.size_bytes).sum();

    if dry_run {
        println!("\n[dry-run] Would clean {} from {} dirs", human_size(total), targets.len());
        return;
    }

    println!("\nCleaning {} from {} dirs...", human_size(total), targets.len());
    let result = clean_all(&targets);
    print_result(result.freed_bytes, result.removed_count, &result.errors);
}

fn cmd_clean_old(args: &[String]) {
    let config = Config::load();
    let dry_run = has_flag(args, &["--dry-run", "-n"]);
    let days = parse_days_flag(args).unwrap_or(config.max_age_days);
    let max_age = Duration::from_secs(days.saturating_mul(SECONDS_PER_DAY));
    let targets = load_targets(args);
    let now = std::time::SystemTime::now();

    let old: Vec<&TargetDir> = targets
        .iter()
        .filter(|t| {
            now.duration_since(t.last_modified)
                .unwrap_or(Duration::ZERO)
                >= max_age
        })
        .collect();

    if old.is_empty() {
        println!("No targets older than {} days.", days);
        return;
    }

    println!("Targets older than {} days:", days);
    for t in &old {
        let age_days = now
            .duration_since(t.last_modified)
            .unwrap_or(Duration::ZERO)
            .as_secs()
            / SECONDS_PER_DAY;
        println!(
            "  {:>8}  {:>3}d  [{}]  {}",
            human_size(t.size_bytes),
            age_days,
            t.kind.label(),
            t.path.display()
        );
    }

    let total: u64 = old.iter().map(|t| t.size_bytes).sum();

    if dry_run {
        println!("\n[dry-run] Would clean {} from {} dirs", human_size(total), old.len());
        return;
    }

    println!("\nCleaning {} from {} dirs...", human_size(total), old.len());
    let result = clean_old(&targets, max_age);
    print_result(result.freed_bytes, result.removed_count, &result.errors);
}

fn print_result(freed: u64, count: usize, errors: &[(std::path::PathBuf, String)]) {
    println!("Done: freed {} from {} dirs ({} errors)", human_size(freed), count, errors.len());
    for (path, err) in errors {
        eprintln!("  error: {}: {}", path.display(), err);
    }
}

fn has_flag(args: &[String], flags: &[&str]) -> bool {
    args.iter().any(|a| flags.contains(&a.as_str()))
}

fn parse_group_flag(args: &[String]) -> Option<ArtifactGroup> {
    for (i, arg) in args.iter().enumerate() {
        if (arg == "--group" || arg == "-g") && i + 1 < args.len() {
            return match args[i + 1].to_lowercase().as_str() {
                "rust" | "r" => Some(ArtifactGroup::Rust),
                "node" | "n" | "node_modules" => Some(ArtifactGroup::NodeModules),
                "build" | "b" => Some(ArtifactGroup::BuildOutput),
                _ => None,
            };
        }
    }
    None
}

fn parse_days_flag(args: &[String]) -> Option<u64> {
    for (i, arg) in args.iter().enumerate() {
        if (arg == "--days" || arg == "-d") && i + 1 < args.len() {
            return args[i + 1].parse().ok();
        }
    }
    None
}

fn print_table(targets: &[TargetDir]) {
    if targets.is_empty() {
        println!("No targets found.");
        return;
    }

    let mut total: u64 = 0;
    let now = std::time::SystemTime::now();

    for &group in ArtifactGroup::ALL {
        let items: Vec<&TargetDir> = targets.iter().filter(|t| t.kind.group() == group).collect();
        if items.is_empty() {
            continue;
        }
        let group_size: u64 = items.iter().map(|t| t.size_bytes).sum();
        total += group_size;
        println!("\n{} — {}", group.label(), human_size(group_size));
        for t in &items {
            let age_days = now
                .duration_since(t.last_modified)
                .unwrap_or(Duration::ZERO)
                .as_secs()
                / SECONDS_PER_DAY;
            println!(
                "  {:>8}  {:>3}d  [{}]  {}",
                human_size(t.size_bytes),
                age_days,
                t.kind.label(),
                t.path.display()
            );
        }
    }
    println!("\nTotal: {} in {} targets", human_size(total), targets.len());
}

fn print_help() {
    println!(
        "WD-40 — dev artifact cleaner

USAGE:
    wd40 [command] [options]

COMMANDS:
    scan, s            Scan and display all artifact directories (default)
    clean, c           Remove all artifact directories
    clean-old, co      Remove artifacts older than N days
    help, -h           Show this help

OPTIONS:
    -g, --group <type> Filter by group: rust, node, build
    -d, --days <N>     Age threshold for clean-old (default: from config)
    -n, --dry-run      Show what would be cleaned without deleting

EXAMPLES:
    wd40                     Scan and show all artifacts
    wd40 scan -g rust        Show only Rust targets
    wd40 clean -g node       Clean all node_modules
    wd40 clean-old -d 14     Clean artifacts older than 14 days
    wd40 clean --dry-run     Preview what would be cleaned

CONFIG:
    ~/.config/wd-40/config.toml"
    );
}
