//! Interactive shell, started by running `hashguard` without arguments.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use colored::Colorize;
use hashguard_core::baseline::{self, Baseline};
use hashguard_core::hashing;
use hashguard_core::heuristics::Level;
use hashguard_core::scan::{self, Control, Outcome, ScanOptions};
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::config::CompletionType;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::FileHistory;
use rustyline::validate::Validator;
use rustyline::{Config, Context as RlContext, Editor, Helper};

use crate::report;

/// Set by Ctrl-C while a scan is running.
static CANCEL: AtomicBool = AtomicBool::new(false);

/// Slash commands: name, argument usage, description.
const COMMANDS: &[(&str, &str, &str)] = &[
    (
        "/scan",
        "[path] [--all] [--blocklist file]",
        "Scan a folder or file (default: System32)",
    ),
    ("/hash", "<file>", "Print the SHA-256 of one file"),
    ("/baseline", "<name> [path]", "Save a baseline of a folder"),
    (
        "/verify",
        "<name> [path]",
        "Compare a folder with a saved baseline",
    ),
    ("/baselines", "", "List saved baselines"),
    (
        "/last",
        "[danger|warn|info|all]",
        "Show the last scan result again",
    ),
    ("/export", "<file>", "Write the last scan as JSON"),
    ("/cd", "<folder>", "Change the working folder"),
    ("/clear", "", "Clear the screen"),
    ("/help", "", "List the commands"),
    ("/exit", "", "Quit"),
];

enum Flow {
    Continue,
    Quit,
}

struct Last {
    path: PathBuf,
    outcome: Outcome,
}

struct Shell {
    last: Option<Last>,
}

pub fn run() -> Result<ExitCode> {
    // Colors and cursor control need virtual terminal mode on older consoles.
    let _ = colored::control::set_virtual_terminal(true);
    let _ = ctrlc::set_handler(|| CANCEL.store(true, Ordering::SeqCst));

    let mut shell = Shell { last: None };
    shell.banner();

    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();
    let mut rl: Editor<ShellHelper, FileHistory> = Editor::with_config(config)?;
    rl.set_helper(Some(ShellHelper {
        files: FilenameCompleter::new(),
    }));
    let history = data_dir().join("history.txt");
    let _ = rl.load_history(&history);

    let mut interrupted = false;
    loop {
        println!("{}", "─".repeat(term_width()).dimmed());
        match rl.readline("› ") {
            Ok(line) => {
                interrupted = false;
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(line);
                match shell.dispatch(line) {
                    Ok(Flow::Continue) => {}
                    Ok(Flow::Quit) => break,
                    Err(e) => println!("{} {e:#}", "error:".red().bold()),
                }
            }
            Err(ReadlineError::Interrupted) => {
                if interrupted {
                    break;
                }
                interrupted = true;
                println!("{}", "Press Ctrl-C again or type /exit to quit".dimmed());
            }
            Err(ReadlineError::Eof) => break,
            Err(e) => return Err(e.into()),
        }
    }

    let _ = std::fs::create_dir_all(data_dir());
    let _ = rl.save_history(&history);
    Ok(ExitCode::SUCCESS)
}

impl Shell {
    fn dispatch(&mut self, line: &str) -> Result<Flow> {
        let args = split_args(line);
        let Some(first) = args.first() else {
            return Ok(Flow::Continue);
        };
        let name = first.trim_start_matches('/').to_lowercase();
        let rest = &args[1..];
        match name.as_str() {
            "scan" => self.scan(rest)?,
            "hash" => hash(rest)?,
            "baseline" => baseline_save(rest)?,
            "verify" => baseline_verify(rest)?,
            "baselines" => baseline_list()?,
            "last" => self.show_last(rest)?,
            "export" => self.export(rest)?,
            "cd" => change_dir(rest)?,
            "clear" => print!("\x1b[2J\x1b[H"),
            "help" | "?" => help(),
            "exit" | "quit" | "q" => return Ok(Flow::Quit),
            _ => println!(
                "Unknown command '{}'. Type {} for the list.",
                first,
                "/help".bold()
            ),
        }
        Ok(Flow::Continue)
    }

    fn scan(&mut self, args: &[String]) -> Result<()> {
        let mut path = None;
        let mut show_all = false;
        let mut blocklist = None;
        let mut it = args.iter();
        while let Some(a) = it.next() {
            match a.as_str() {
                "--all" => show_all = true,
                "--blocklist" => {
                    blocklist = Some(PathBuf::from(
                        it.next().context("--blocklist needs a file")?,
                    ))
                }
                o if o.starts_with("--") => bail!("unknown option {o}"),
                p => path = Some(PathBuf::from(p)),
            }
        }
        let path = path.unwrap_or_else(default_root);
        let opts = ScanOptions {
            blocklist: match blocklist {
                Some(p) => scan::load_blocklist(&p)?,
                None => Default::default(),
            },
        };

        CANCEL.store(false, Ordering::SeqCst);
        println!(
            "{}",
            format!("Scanning {} · Ctrl-C to stop", path.display()).dimmed()
        );
        let started = Instant::now();
        let draw = |done: usize, total: usize| {
            if total == 0 {
                return;
            }
            let step = (total / 200).max(1);
            if !done.is_multiple_of(step) && done != total {
                return;
            }
            let frac = done as f64 / total as f64;
            let filled = (frac * 24.0) as usize;
            eprint!(
                "\r  {}{} {:>3}%  {} / {} files ",
                "█".repeat(filled),
                "░".repeat(24 - filled),
                (frac * 100.0) as usize,
                num(done),
                num(total)
            );
        };
        let ctl = Control {
            cancel: &CANCEL,
            on_progress: &draw,
        };
        let out = scan::scan_with(&path, &opts, &ctl)?;
        eprint!("\r{}\r", " ".repeat(70));

        let min = if show_all { Level::Ok } else { Level::Warn };
        report::print_scan_from(&out.reports, &out.failures, min);
        let hidden = out
            .reports
            .iter()
            .filter(|r| r.level == Level::Info && min > Level::Info)
            .count();
        if hidden > 0 {
            println!(
                "{}",
                format!("{hidden} informational findings hidden, /last info shows them").dimmed()
            );
        }
        if out.cancelled {
            println!("{}", "Stopped early, results are partial.".yellow());
        }
        println!(
            "{}",
            format!("Finished in {:.1}s", started.elapsed().as_secs_f32()).dimmed()
        );

        let danger = out
            .reports
            .iter()
            .filter(|r| r.level == Level::Danger)
            .count();
        let warn = out
            .reports
            .iter()
            .filter(|r| r.level == Level::Warn)
            .count();
        remember(&format!(
            "scan {} · {} files · {danger} danger, {warn} warn",
            path.display(),
            num(out.reports.len())
        ));
        self.last = Some(Last { path, outcome: out });
        Ok(())
    }

    fn show_last(&self, args: &[String]) -> Result<()> {
        let Some(last) = &self.last else {
            bail!("nothing scanned yet, run /scan first");
        };
        let min = match args.first().map(|s| s.to_lowercase()).as_deref() {
            None | Some("info") => Level::Info,
            Some("danger") => Level::Danger,
            Some("warn") => Level::Warn,
            Some("all") | Some("ok") => Level::Ok,
            Some(other) => bail!("unknown level '{other}', use danger, warn, info or all"),
        };
        println!("{}", format!("Last scan: {}", last.path.display()).dimmed());
        report::print_scan_from(&last.outcome.reports, &last.outcome.failures, min);
        Ok(())
    }

    fn export(&self, args: &[String]) -> Result<()> {
        let file = args.first().context("usage: /export <file>")?;
        let Some(last) = &self.last else {
            bail!("nothing scanned yet, run /scan first");
        };
        let json = serde_json::to_string_pretty(&last.outcome.reports)?;
        std::fs::write(file, json).with_context(|| format!("cannot write {file}"))?;
        println!(
            "Saved {} results to {file}",
            num(last.outcome.reports.len())
        );
        Ok(())
    }

    fn banner(&self) {
        let width = term_width();
        let inner = width - 2;
        let left_w = 40.min(inner / 2);
        let right_w = inner - left_w - 1;

        let title = format!(" hashguard v{} ", env!("CARGO_PKG_VERSION"));
        println!(
            "{}",
            format!(
                "┌─{title}{}┐",
                "─".repeat(inner - 1 - title.chars().count())
            )
            .dimmed()
        );

        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let mut left: Vec<(String, Kind)> = vec![
            ("Welcome back!".into(), Kind::Bold),
            (String::new(), Kind::Plain),
        ];
        left.extend(pixel_art(SHIELD).into_iter().map(|l| (l, Kind::Plain)));
        left.extend([
            (String::new(), Kind::Plain),
            (format!("Windows · {threads} threads"), Kind::Dim),
            (cwd, Kind::Dim),
        ]);

        let mut right: Vec<(String, Kind)> = vec![
            ("Tips for getting started".into(), Kind::Bold),
            ("Type /scan to check System32".into(), Kind::Plain),
            (
                "/baseline <name> saves a clean snapshot".into(),
                Kind::Plain,
            ),
            ("Tab completes, ↑ recalls history".into(), Kind::Plain),
            ("─".repeat(right_w.saturating_sub(2)), Kind::Dim),
            ("Recent activity".into(), Kind::Bold),
        ];
        let recent = recent_lines();
        if recent.is_empty() {
            right.push(("No recent activity".into(), Kind::Dim));
        } else {
            right.extend(recent.into_iter().map(|l| (l, Kind::Plain)));
        }

        for i in 0..left.len().max(right.len()) {
            let (l, lk) = left.get(i).cloned().unwrap_or((String::new(), Kind::Plain));
            let (r, rk) = right
                .get(i)
                .cloned()
                .unwrap_or((String::new(), Kind::Plain));
            let l = center(&fit(&l, left_w), left_w);
            let r = format!(" {}", fit(&r, right_w - 1));
            let r = format!("{r}{}", " ".repeat(right_w - r.chars().count()));
            println!(
                "{}{}{}{}{}",
                "│".dimmed(),
                lk.paint(&l),
                "│".dimmed(),
                rk.paint(&r),
                "│".dimmed()
            );
        }
        println!("{}", format!("└{}┘", "─".repeat(inner)).dimmed());
        println!(
            "{}",
            "  /help for commands · Tab to complete · Ctrl-C twice to exit".dimmed()
        );
        println!();
    }
}

/// Pixel grid of the logo: '#' is a lit pixel, two rows share one text line.
const SHIELD: &[&str] = &[
    ".#########.",
    "###########",
    "########.##",
    "#######.###",
    "##.###.####",
    "###.#.#####",
    ".###.#####.",
    "..#######..",
    "...#####...",
    "....###....",
];

/// Turn a pixel grid into text with half-block characters.
fn pixel_art(rows: &[&str]) -> Vec<String> {
    rows.chunks(2)
        .map(|pair| {
            let top = pair[0].as_bytes();
            let bottom = pair.get(1).map(|r| r.as_bytes()).unwrap_or(&[]);
            (0..top.len())
                .map(|x| match (top[x] == b'#', bottom.get(x) == Some(&b'#')) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (false, false) => ' ',
                })
                .collect()
        })
        .collect()
}

#[derive(Clone, Copy)]
enum Kind {
    Plain,
    Bold,
    Dim,
}

impl Kind {
    fn paint(self, s: &str) -> String {
        match self {
            Kind::Plain => s.to_string(),
            Kind::Bold => s.bold().to_string(),
            Kind::Dim => s.dimmed().to_string(),
        }
    }
}

fn hash(args: &[String]) -> Result<()> {
    let file = args.first().context("usage: /hash <file>")?;
    let digest =
        hashing::sha256_file(Path::new(file)).with_context(|| format!("cannot read {file}"))?;
    println!("{digest}  {file}");
    Ok(())
}

fn baseline_save(args: &[String]) -> Result<()> {
    let name = args.first().context("usage: /baseline <name> [path]")?;
    let file = baseline_file(name)?;
    let path = args.get(1).map(PathBuf::from).unwrap_or_else(default_root);
    println!("{}", format!("Hashing {} ...", path.display()).dimmed());
    let (files, errors) = scan::hash_tree(&path)?;
    let count = files.len();
    Baseline::new(&path, files).save(&file)?;
    println!("Baseline '{name}' saved: {} files", num(count));
    if !errors.is_empty() {
        println!("{} files could not be read and were skipped", errors.len());
    }
    remember(&format!("baseline {name} · {} files", num(count)));
    Ok(())
}

fn baseline_verify(args: &[String]) -> Result<()> {
    let name = args.first().context("usage: /verify <name> [path]")?;
    let saved = Baseline::load(&baseline_file(name)?)
        .with_context(|| format!("no baseline named '{name}'"))?;
    let path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&saved.root));
    println!("{}", format!("Hashing {} ...", path.display()).dimmed());
    let (current, _errors) = scan::hash_tree(&path)?;
    let diff = baseline::diff(&saved, &current);
    report::print_diff(&diff);
    remember(&format!(
        "verify {name} · {} modified, {} added, {} removed",
        diff.modified.len(),
        diff.added.len(),
        diff.removed.len()
    ));
    Ok(())
}

fn baseline_list() -> Result<()> {
    let dir = baselines_dir();
    let mut rows = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            if let (Some(name), Ok(b)) =
                (p.file_stem().and_then(|s| s.to_str()), Baseline::load(&p))
            {
                rows.push((b.created_unix, name.to_string(), b.files.len(), b.root));
            }
        }
    }
    if rows.is_empty() {
        println!("No baselines yet. Try {}", "/baseline <name>".bold());
        return Ok(());
    }
    rows.sort_by_key(|r| std::cmp::Reverse(r.0));
    for (created, name, files, root) in rows {
        println!(
            "{:<20} {:>8} files  {}  {}",
            name.bold(),
            num(files),
            age(created).dimmed(),
            root.dimmed()
        );
    }
    Ok(())
}

fn change_dir(args: &[String]) -> Result<()> {
    let dir = args.first().context("usage: /cd <folder>")?;
    std::env::set_current_dir(dir).with_context(|| format!("cannot open {dir}"))?;
    println!("{}", std::env::current_dir()?.display());
    Ok(())
}

fn help() {
    println!("{}", "Commands".bold());
    for (name, usage, desc) in COMMANDS {
        println!("  {:<11} {:<38} {}", name.bold(), usage.dimmed(), desc);
    }
}

/// Split a line into words, keeping "quoted paths" together.
fn split_args(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    out.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            c => {
                cur.push(c);
                started = true;
            }
        }
    }
    if started {
        out.push(cur);
    }
    out
}

fn num(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn age(unix: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let s = now.saturating_sub(unix);
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", s / 60),
        3600..=86399 => format!("{} h ago", s / 3600),
        _ => format!("{} days ago", s / 86400),
    }
}

fn term_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(100)
        .clamp(70, 110)
}

/// Cut `s` to `w` characters, ending with an ellipsis when it is longer.
fn fit(s: &str, w: usize) -> String {
    if s.chars().count() <= w {
        return s.to_string();
    }
    let keep: String = s.chars().take(w.saturating_sub(1)).collect();
    format!("{keep}…")
}

fn center(s: &str, w: usize) -> String {
    let len = s.chars().count();
    let left = w.saturating_sub(len) / 2;
    format!(
        "{}{s}{}",
        " ".repeat(left),
        " ".repeat(w.saturating_sub(len + left))
    )
}

fn default_root() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(|p| PathBuf::from(p).join("System32"))
        .unwrap_or_else(|| PathBuf::from("C:\\Windows\\System32"))
}

/// Same folder the desktop panel uses, so both tools share baselines.
fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("io.github.wirekits.hashguard")
}

fn baselines_dir() -> PathBuf {
    data_dir().join("baselines")
}

fn baseline_file(name: &str) -> Result<PathBuf> {
    let name = name.trim();
    let ok = !name.is_empty()
        && name.chars().count() <= 64
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_');
    if !ok {
        bail!("name: up to 64 characters; letters, digits, space, '-' and '_' only");
    }
    std::fs::create_dir_all(baselines_dir())?;
    Ok(baselines_dir().join(format!("{name}.json")))
}

/// Append a line to the activity log shown on the banner.
fn remember(text: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = std::fs::create_dir_all(data_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir().join("recent.txt"))
    {
        use std::io::Write;
        let _ = writeln!(f, "{now}\t{text}");
    }
}

fn recent_lines() -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(data_dir().join("recent.txt")) else {
        return Vec::new();
    };
    let mut lines: Vec<String> = text
        .lines()
        .rev()
        .take(3)
        .filter_map(|l| {
            let (t, msg) = l.split_once('\t')?;
            Some(format!("{msg} · {}", age(t.parse().ok()?)))
        })
        .collect();
    lines.truncate(3);
    lines
}

struct ShellHelper {
    files: FilenameCompleter,
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &RlContext<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let head = &line[..pos];
        if !head.contains(char::is_whitespace) {
            let word = head.trim_start_matches('/').to_lowercase();
            let items = COMMANDS
                .iter()
                .filter(|(n, _, _)| n[1..].starts_with(&word))
                .map(|(n, _, d)| Pair {
                    display: format!("{n:<11} {d}"),
                    replacement: n.to_string(),
                })
                .collect();
            return Ok((0, items));
        }
        self.files.complete(line, pos, ctx)
    }
}

impl Hinter for ShellHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &RlContext<'_>) -> Option<String> {
        if pos < line.len() || line.len() < 2 || line.contains(' ') || !line.starts_with('/') {
            return None;
        }
        let (name, usage, _) = COMMANDS.iter().find(|(n, _, _)| n.starts_with(line))?;
        let rest = &name[line.len()..];
        Some(if usage.is_empty() {
            rest.to_string()
        } else {
            format!("{rest} {usage}")
        })
    }
}

impl Highlighter for ShellHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let end = line.find(' ').unwrap_or(line.len());
        let (word, rest) = line.split_at(end);
        if COMMANDS.iter().any(|(n, _, _)| *n == word) {
            Cow::Owned(format!("{}{rest}", word.bold()))
        } else {
            Cow::Borrowed(line)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(hint.dimmed().to_string())
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        true
    }
}

impl Validator for ShellHelper {}
impl Helper for ShellHelper {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_keeps_quoted_paths_together() {
        assert_eq!(
            split_args(r#"/scan "C:\Program Files" --all"#),
            vec!["/scan", r"C:\Program Files", "--all"]
        );
    }

    #[test]
    fn split_ignores_extra_spaces() {
        assert_eq!(split_args("  /hash   a.exe  "), vec!["/hash", "a.exe"]);
    }

    #[test]
    fn numbers_get_thousands_separators() {
        assert_eq!(num(0), "0");
        assert_eq!(num(999), "999");
        assert_eq!(num(6210), "6,210");
        assert_eq!(num(1234567), "1,234,567");
    }

    #[test]
    fn pixel_art_pairs_rows_into_half_blocks() {
        assert_eq!(pixel_art(&["#.#.", "##.."]), vec!["█▄▀ "]);
    }

    #[test]
    fn shield_is_five_lines_of_equal_width() {
        let art = pixel_art(SHIELD);
        assert_eq!(art.len(), 5);
        assert!(art.iter().all(|l| l.chars().count() == 11));
    }

    #[test]
    fn fit_adds_an_ellipsis() {
        assert_eq!(fit("abcdef", 4), "abc…");
        assert_eq!(fit("abc", 4), "abc");
    }

    #[test]
    fn baseline_names_are_restricted() {
        assert!(baseline_file("system32-clean").is_ok());
        assert!(baseline_file("..\\evil").is_err());
        assert!(baseline_file("").is_err());
    }
}
