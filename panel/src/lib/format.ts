import type { Level, SigStatus } from "./types";

export const levelLabel: Record<Level, string> = {
  Ok: "Clean",
  Info: "Info",
  Warn: "Warning",
  Danger: "Danger",
};

export const sigLabel: Record<SigStatus, string> = {
  Valid: "Signed",
  Unsigned: "Unsigned",
  Invalid: "Invalid signature",
  Error: "Not checked",
};

export const fmtNum = (n: number) => n.toLocaleString("en-US");

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)} s`;
  return `${Math.floor(s / 60)} min ${Math.round(s % 60)} s`;
}

export function formatDate(unix: number): string {
  return new Date(unix * 1000).toLocaleString("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

export function timeAgo(ms: number): string {
  const s = Math.round((Date.now() - ms) / 1000);
  if (s < 45) return "just now";
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h} h ago`;
  return new Date(ms).toLocaleDateString("en-US", { dateStyle: "medium" });
}

/** Split `C:\a\b\file.dll` into directory and file name. */
export function splitPath(p: string): { dir: string; name: string } {
  const i = Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/"));
  return i < 0 ? { dir: "", name: p } : { dir: p.slice(0, i), name: p.slice(i + 1) };
}

export function pluralFiles(n: number): string {
  return `${fmtNum(n)} ${n === 1 ? "file" : "files"}`;
}

export const pct = (n: number, total: number) =>
  total ? `${((n / total) * 100).toLocaleString("en-US", { maximumFractionDigits: 1 })}%` : "0%";
