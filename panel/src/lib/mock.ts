// Dev-only fake backend so the UI can be developed and screenshotted in a plain
// browser (`npm run dev`). Imported only when not running inside Tauri and only
// in dev builds, so it never ships in the .exe.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import type { BaselineInfo, FileReport, Level, SigInfo } from "./types";

function fakeHash(seed: string): string {
  let h = 2166136261;
  let out = "";
  for (let i = 0; out.length < 64; i++) {
    h ^= seed.charCodeAt(i % seed.length) + i;
    h = Math.imul(h, 16777619) >>> 0;
    out += h.toString(16).padStart(8, "0");
  }
  return out.slice(0, 64);
}

const ms: SigInfo = { status: "Valid", signer: "Microsoft Windows", via_catalog: true, detail: null };

function file(path: string, level: Level, sig: Partial<SigInfo>, findings: string[] = []): FileReport {
  return {
    path,
    size: 40_000 + (fakeHash(path).charCodeAt(3) * 9_731) % 4_000_000,
    sha256: fakeHash(path),
    signature: { ...ms, ...sig },
    level,
    findings,
  };
}

const sys = "C:\\Windows\\System32\\";
const names = [
  "kernel32.dll", "ntdll.dll", "user32.dll", "gdi32.dll", "advapi32.dll", "shell32.dll",
  "ole32.dll", "combase.dll", "rpcrt4.dll", "sechost.dll", "bcrypt.dll", "crypt32.dll",
  "wintrust.dll", "ws2_32.dll", "winhttp.dll", "dwmapi.dll", "uxtheme.dll", "notepad.exe",
  "cmd.exe", "conhost.exe", "svchost.exe", "lsass.exe", "services.exe", "taskmgr.exe",
  "explorer.exe", "msiexec.exe", "regedit.exe", "powershell.exe", "dxgi.dll", "d3d11.dll",
];

const reports: FileReport[] = [
  file("C:\\Users\\Public\\Downloads\\svchost.exe", "Danger", { status: "Unsigned", signer: null, via_catalog: false }, [
    "'svchost.exe' is a core Windows process name but lives outside the system directories",
    "unsigned binary in a user-writable location",
  ]),
  file(sys + "drivers\\netflt.sys", "Danger", { status: "Invalid", detail: "file digest does not match the signature (modified after signing)" }, [
    "signature check failed: file digest does not match the signature (modified after signing)",
  ]),
  file("C:\\Users\\me\\AppData\\Local\\Temp\\setup_helper.exe", "Warn", { status: "Unsigned", signer: null, via_catalog: false }, [
    "unsigned binary in a user-writable location",
  ]),
  file("C:\\Users\\me\\Downloads\\invoice_2026.pdf.exe", "Warn", { status: "Valid", signer: "Acme Tools LLC", via_catalog: false }, [
    "double extension imitating a document or media file",
  ]),
  file(sys + "DMRCDecoder.dll", "Warn", { status: "Invalid", signer: "Microsoft Azure Code Sign", via_catalog: false, detail: "certificate chain could not be built (unknown or missing root; signer name is unverified)" }, [
    "signature check failed: certificate chain could not be built (unknown or missing root; signer name is unverified)",
  ]),
  ...["nvapi64.dll", "nvcuda.dll", "vulkan-1.dll"].map((n) =>
    file(sys + n, "Info", { signer: n.startsWith("vulkan") ? "LunarG, Inc." : "NVIDIA Corporation", via_catalog: false }, [
      `signed by a non-Microsoft publisher (${n.startsWith("vulkan") ? "LunarG, Inc." : "NVIDIA Corporation"}) in a system directory`,
    ]),
  ),
  ...Array.from({ length: 64 }, (_, i) => {
    const vendors = [
      ["Google LLC", "C:\\Program Files\\Google\\Chrome\\Application\\"],
      ["Intel Corporation", "C:\\Program Files\\Intel\\Wifi\\bin\\"],
      ["Realtek Semiconductor Corp.", "C:\\Program Files\\Realtek\\Audio\\HDA\\"],
      ["Mozilla Corporation", "C:\\Program Files\\Mozilla Firefox\\"],
    ] as const;
    const [signer, dir] = vendors[i % vendors.length];
    return file(`${dir}module_${i}.dll`, "Ok", { signer, via_catalog: false });
  }),
  ...Array.from({ length: 1176 }, (_, i) => {
    const n = names[i % names.length];
    const dir = i < names.length ? sys : `${sys}${["en-US", "wbem", "drivers", "WinMetadata", "migration"][i % 5]}\\`;
    const name = i < names.length ? n : n.replace(/(\.\w+)$/, `_${i}$1`);
    return file(dir + name, "Ok", {});
  }),
];

let baselines: BaselineInfo[] = [
  { name: "system32-clean", root: "C:\\Windows\\System32", created_unix: 1795132800, files: 6210 },
  { name: "program-files", root: "C:\\Program Files", created_unix: 1794960000, files: 2874 },
];

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export function installMocks() {
  mockWindows("main");
  mockIPC(
    async (cmd, args) => {
      const a = (args ?? {}) as Record<string, string>;
      switch (cmd) {
        case "backdrop":
          return "solid";
        case "system_info":
          return { os: "Windows 11", build: 26300, cores: 16, backdrop: "solid" };
        case "quick_paths":
          return [
            { label: "System32", path: "C:\\Windows\\System32" },
            { label: "Downloads", path: "C:\\Users\\me\\Downloads" },
            { label: "Desktop", path: "C:\\Users\\me\\Desktop" },
            { label: "Temp", path: "C:\\Users\\me\\AppData\\Local\\Temp" },
            { label: "Program Files", path: "C:\\Program Files" },
          ];
        case "start_scan": {
          const total = reports.length;
          for (let done = 0; done <= total; done += 60) {
            await emit("scan-progress", { done, total });
            await sleep(40);
          }
          await emit("scan-progress", { done: total, total });
          return { reports, failures: [], cancelled: false, elapsed_ms: 8412 };
        }
        case "list_baselines":
          return baselines;
        case "create_baseline": {
          const b = { name: a.name, root: a.path, created_unix: Math.floor(Date.now() / 1000), files: 1248 };
          baselines = [b, ...baselines];
          return b;
        }
        case "verify_baseline":
          await sleep(600);
          return {
            baseline: baselines.find((b) => b.name === a.name) ?? baselines[0],
            root: "C:\\Windows\\System32",
            diff: {
              modified: [{ path: "drivers/netflt.sys", old_sha256: fakeHash("old"), new_sha256: fakeHash("new") }],
              added: ["tasks/updater_svc.exe"],
              removed: [],
            },
          };
        case "delete_baseline":
          baselines = baselines.filter((b) => b.name !== a.name);
          return null;
        case "plugin:window|is_maximized":
          return false;
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
