# hashguard

A small Windows command-line tool, written in Rust, that checks whether the
executables on your machine are still what they claim to be.

It combines three independent checks:

1. **Authenticode verification.** Calls `WinVerifyTrust` directly. Most Windows
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

## Why this works

Modern Windows binaries are signed. Changing even one byte of a signed file
breaks its digest, and an attacker cannot produce a valid Microsoft signature for
altered code. So a mismatch is a strong signal; hashguard reports it as `DANGER`.

## What it is not

hashguard is an integrity and triage tool, **not an antivirus**. A file with no
findings is not proven safe (malware can be validly signed with a stolen or
purchased certificate), and a finding means "inspect this", not "this is malware".
It also does not read file contents for malicious behaviour.

## Usage

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
(`verify`), so it can be used in scripts and CI.

### Example

```
[DANGER] C:\Users\bob\AppData\Local\Temp\svchost.exe
         sha256 621ba393...
         unsigned
         - 'svchost.exe' is a core Windows process name but lives outside the system directories
```

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

Requires Rust (stable) and Windows.

```powershell
cargo build --release
cargo test
```

The binary is `target\release\hashguard.exe`. Hashing, baselines and heuristics
build and test on any OS; signature verification is Windows-only.

## Project layout

| File                  | Purpose                                              |
|-----------------------|------------------------------------------------------|
| `src/signature.rs`    | `WinVerifyTrust` + catalog lookup via `windows-sys`  |
| `src/heuristics.rs`   | Detection rules and severity levels                  |
| `src/baseline.rs`     | Snapshot format and diffing                          |
| `src/scan.rs`         | Parallel directory scanning (rayon)                  |
| `src/pe.rs`           | PE header detection (`MZ` / `PE\0\0`)                |
| `src/hashing.rs`      | Streaming SHA-256                                    |

## License

MIT
