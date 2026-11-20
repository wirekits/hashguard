//! Terminal output.

use crate::baseline::Diff;
use crate::heuristics::Level;
use crate::scan::FileReport;
use crate::signature::SigStatus;
use colored::Colorize;

fn level_tag(level: Level) -> colored::ColoredString {
    match level {
        Level::Ok => "  OK  ".green(),
        Level::Info => " INFO ".cyan(),
        Level::Warn => " WARN ".yellow().bold(),
        Level::Danger => "DANGER".red().bold(),
    }
}

fn sig_text(r: &FileReport) -> String {
    let base = match r.signature.status {
        SigStatus::Valid => "signed",
        SigStatus::Unsigned => "unsigned",
        SigStatus::Invalid => "INVALID",
        SigStatus::Error => "unknown",
    };
    let via = if r.signature.via_catalog {
        " (catalog)"
    } else {
        ""
    };
    match &r.signature.signer {
        Some(s) => format!("{base}{via} - {s}"),
        None => format!("{base}{via}"),
    }
}

pub fn print_scan(reports: &[FileReport], errors: &[(String, String)], show_all: bool) {
    for r in reports {
        if !show_all && r.level == Level::Ok {
            continue;
        }
        println!("[{}] {}", level_tag(r.level), r.path);
        println!("         sha256 {}", r.sha256.dimmed());
        println!("         {}", sig_text(r));
        for f in &r.findings {
            println!("         {} {f}", "-".dimmed());
        }
    }

    let count = |l: Level| reports.iter().filter(|r| r.level == l).count();
    println!();
    println!(
        "{} files scanned: {} ok, {} info, {} warnings, {} danger",
        reports.len(),
        count(Level::Ok),
        count(Level::Info),
        count(Level::Warn),
        count(Level::Danger),
    );
    if !errors.is_empty() {
        println!("{} files could not be read:", errors.len());
        for (p, e) in errors.iter().take(10) {
            println!("  {p}: {e}");
        }
        if errors.len() > 10 {
            println!("  ... and {} more", errors.len() - 10);
        }
    }
}

pub fn print_diff(diff: &Diff) {
    if diff.is_clean() {
        println!("{}", "No changes against the baseline.".green());
        return;
    }
    for m in &diff.modified {
        println!("{} {}", "MODIFIED".red().bold(), m.path);
        println!("         was {}", m.old_sha256.dimmed());
        println!("         now {}", m.new_sha256.dimmed());
    }
    for a in &diff.added {
        println!("{}    {a}", "ADDED".yellow().bold());
    }
    for r in &diff.removed {
        println!("{}  {r}", "REMOVED".cyan());
    }
    println!();
    println!(
        "{} modified, {} added, {} removed",
        diff.modified.len(),
        diff.added.len(),
        diff.removed.len()
    );
}
