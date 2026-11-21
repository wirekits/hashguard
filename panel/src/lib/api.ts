import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { BaselineInfo, QuickPath, ScanResult, SystemInfo, VerifyResult } from "./types";

export async function pickFolder(title: string): Promise<string | null> {
  const r = await open({ directory: true, multiple: false, title });
  return typeof r === "string" ? r : null;
}

export async function pickFile(title: string): Promise<string | null> {
  const r = await open({
    directory: false,
    multiple: false,
    title,
    filters: [{ name: "Text", extensions: ["txt", "csv", "lst"] }],
  });
  return typeof r === "string" ? r : null;
}

export const quickPaths = () => invoke<QuickPath[]>("quick_paths");

/** "mica" on Windows 11, otherwise "solid". */
export const backdrop = () => invoke<"mica" | "solid">("backdrop");

export const systemInfo = () => invoke<SystemInfo>("system_info");

export const startScan = (path: string, blocklist: string | null) =>
  invoke<ScanResult>("start_scan", { path, blocklist });

export const cancelScan = () => invoke<void>("cancel_scan");

export const listBaselines = () => invoke<BaselineInfo[]>("list_baselines");

export const createBaseline = (name: string, path: string) =>
  invoke<BaselineInfo>("create_baseline", { name, path });

export const verifyBaseline = (name: string) => invoke<VerifyResult>("verify_baseline", { name });

export const deleteBaseline = (name: string) => invoke<void>("delete_baseline", { name });

export const revealInExplorer = (path: string) => invoke<void>("reveal_in_explorer", { path });

/** Ask where to save, then write the text. Returns the chosen path or null. */
export async function saveTextFile(suggested: string, content: string): Promise<string | null> {
  const path = await save({
    defaultPath: suggested,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return null;
  await invoke("save_text", { path, content });
  return path;
}
