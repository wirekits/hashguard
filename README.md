# hashguard

A Windows tool, written in Rust, that checks whether the executables on your
machine are still what they claim to be. It ships as two programs built on one
core library:

- **`hashguard.exe`**, a command-line tool for scripts and CI;
- **`hashguard-panel.exe`**, a desktop app (Tauri + Svelte) styled after
  Windows 11, with a results dashboard.

## What it checks

1. **Authenticode signatures.** Calls `WinVerifyTrust` directly. Most Windows
   system files are not signed individually; their hashes are listed in signed
   catalog (`.cat`) files, so hashguard falls back to a catalog lookup, the same
   way Windows itself does.
2. **Baselines.** Snapshot the SHA-256 of every `.exe`/`.dll`/`.sys` in a folder
   while the machine is clean, then diff against it later to see what was
   modified, added or removed.
3. **Heuristics.** Rules for the things that actually give malware away:
   a process named `svchost.exe` living in `%TEMP%`, a "system" file signed by
   someone other than Microsoft, a signature that no longer matches the file,
   `invoice.pdf.exe` style names, unsigned binaries in user-writable folders.
   You can also supply a blocklist of known-bad hashes.

Modern Windows binaries are signed. Changing even one byte of a signed file
breaks its digest, and an attacker cannot produce a valid Microsoft signature
for altered code, so a mismatch is reported as `DANGER`.

## What it is not

hashguard is an integrity and triage tool, **not an antivirus**. A file with no
findings is not proven safe (malware can be validly signed with a stolen or
purchased certificate), and a finding means "inspect this", not "this is
malware". It does not analyse what a file does.

## Desktop panel

- Windows 11 look: Mica backdrop, navigation pane, light and dark theme that
  follow the system (solid background on Windows 10).
- Scan dashboard: verdict banner, headline tiles, distribution by severity,
  top signing publishers, and a filterable, searchable file list with a details
  pane (copy hash, show in Explorer).
- Live progress with cancel, JSON export, blocklist support.
- Baselines: create, verify with a clear diff, delete.
- Interface in Russian.

## Command line

```powershell
# Scan a folder; only files with findings are shown
hashguard scan C:\Windows\System32

# Show everything, or get JSON for scripts
hashguard scan C:\Users\me\Downloads --all
hashguard scan C:\Users\me\Downloads --json

# Add known-bad hashes (one SHA-256 per line, # comments allowed)
hashguard scan D:\suspect --blocklist bad_hashes.txt

# Snapshot a clean system, then check it later
hashguard baseline C:\Windows\System32 -o clean.json
hashguard verify   C:\Windows\System32 -b clean.json

# Hash a single file
hashguard hash setup.exe
```

Exit codes: `0` clean, `1` error, `2` danger found (`scan`) or differences found
(`verify`).

Scanning a stock `C:\Windows\System32` (about 6,200 binaries) takes roughly 35
seconds in a release build and reports no `DANGER` results.

## Severity levels

| Level    | Meaning                                                              |
|----------|----------------------------------------------------------------------|
| `OK`     | Valid signature, nothing unusual                                     |
| `INFO`   | Worth knowing (unsigned elsewhere, third-party publisher)            |
| `WARN`   | Suspicious: unsigned in a system or user-writable folder, bad chain  |
| `DANGER` | Modified after signing, masquerading as a system file, on blocklist  |

## Build

Requires Windows, Rust (stable) and Node.js LTS.

```powershell
# CLI
cargo build --release -p hashguard

# Desktop panel (needs the Tauri CLI: cargo install tauri-cli)
cd panel
npm ci
cargo tauri build --no-bundle

# Tests and lints
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Binaries land in `target\release\`. For UI work without Rust, `npm run dev` in
`panel/` serves the interface in a browser with a fake backend (dev builds only).

## Project layout

| Path                         | Purpose                                             |
|------------------------------|-----------------------------------------------------|
| `crates/core/`               | Library: hashing, signatures, baselines, heuristics |
| `crates/core/src/signature.rs` | `WinVerifyTrust` + catalog lookup via `windows-sys` |
| `crates/core/src/scan.rs`    | Parallel scanning (rayon) with progress and cancel  |
| `crates/cli/`                | `hashguard` command-line tool                       |
| `panel/src-tauri/`           | Tauri shell: commands over the core, Mica backdrop  |
| `panel/src/`                 | Svelte 5 interface                                  |

## License

MIT
