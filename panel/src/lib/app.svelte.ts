import { listen } from "@tauri-apps/api/event";
import * as api from "./api";
import type {
  BaselineInfo,
  FileReport,
  Level,
  QuickPath,
  ScanResult,
  VerifyResult,
} from "./types";

export type View = "scan" | "baselines" | "about";
export type Filter = "all" | Level;

export const ui = $state<{ view: View }>({ view: "scan" });

export const scan = $state({
  path: "",
  blocklist: "",
  running: false,
  done: 0,
  total: 0,
  result: null as ScanResult | null,
  error: "",
  filter: "all" as Filter,
  query: "",
  selected: null as FileReport | null,
  quick: [] as QuickPath[],
});

export async function loadQuickPaths() {
  try {
    scan.quick = await api.quickPaths();
    if (!scan.path && scan.quick.length) scan.path = scan.quick[0].path;
  } catch {
    /* quick paths are a convenience only */
  }
}

export async function runScan() {
  if (scan.running || !scan.path.trim()) return;
  scan.running = true;
  scan.error = "";
  scan.result = null;
  scan.selected = null;
  scan.done = 0;
  scan.total = 0;
  scan.filter = "all";
  scan.query = "";

  const unlisten = await listen<{ done: number; total: number }>(
    "scan-progress",
    (e) => {
      scan.done = e.payload.done;
      scan.total = e.payload.total;
    },
  );
  try {
    scan.result = await api.startScan(scan.path.trim(), scan.blocklist.trim() || null);
  } catch (e) {
    scan.error = String(e);
  } finally {
    unlisten();
    scan.running = false;
  }
}

export const stopScan = () => api.cancelScan();

export const baselines = $state({
  list: [] as BaselineInfo[],
  name: "",
  path: "",
  busy: "" as "" | "create" | "verify",
  error: "",
  verifying: "",
  result: null as VerifyResult | null,
});

export async function refreshBaselines() {
  try {
    baselines.list = await api.listBaselines();
  } catch (e) {
    baselines.error = String(e);
  }
}

export async function makeBaseline() {
  if (baselines.busy || !baselines.name.trim() || !baselines.path.trim()) return;
  baselines.busy = "create";
  baselines.error = "";
  try {
    await api.createBaseline(baselines.name.trim(), baselines.path.trim());
    baselines.name = "";
    await refreshBaselines();
  } catch (e) {
    baselines.error = String(e);
  } finally {
    baselines.busy = "";
  }
}

export async function runVerify(name: string) {
  if (baselines.busy) return;
  baselines.busy = "verify";
  baselines.verifying = name;
  baselines.error = "";
  baselines.result = null;
  try {
    baselines.result = await api.verifyBaseline(name);
  } catch (e) {
    baselines.error = String(e);
  } finally {
    baselines.busy = "";
    baselines.verifying = "";
  }
}

export async function removeBaseline(name: string) {
  try {
    await api.deleteBaseline(name);
    if (baselines.result?.baseline.name === name) baselines.result = null;
    await refreshBaselines();
  } catch (e) {
    baselines.error = String(e);
  }
}
