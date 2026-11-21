// Mirrors the serde output of hashguard-core and the panel's Rust commands.

export type Level = "Ok" | "Info" | "Warn" | "Danger";
export type SigStatus = "Valid" | "Unsigned" | "Invalid" | "Error";

export interface SigInfo {
  status: SigStatus;
  signer: string | null;
  via_catalog: boolean;
  detail: string | null;
}

export interface FileReport {
  path: string;
  size: number;
  sha256: string;
  signature: SigInfo;
  level: Level;
  findings: string[];
}

export interface ScanResult {
  reports: FileReport[];
  failures: [string, string][];
  cancelled: boolean;
  elapsed_ms: number;
}

export interface Progress {
  done: number;
  total: number;
}

export interface QuickPath {
  label: string;
  path: string;
}

export interface BaselineInfo {
  name: string;
  root: string;
  created_unix: number;
  files: number;
}

export interface Modified {
  path: string;
  old_sha256: string;
  new_sha256: string;
}

export interface Diff {
  modified: Modified[];
  added: string[];
  removed: string[];
}

export interface VerifyResult {
  baseline: BaselineInfo;
  root: string;
  diff: Diff;
}
