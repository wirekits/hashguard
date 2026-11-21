<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    ui,
    scan,
    loadQuickPaths,
    loadSystemInfo,
    refreshBaselines,
    type View,
  } from "./lib/app.svelte";
  import { backdrop } from "./lib/api";
  import { fmtNum, timeAgo } from "./lib/format";
  import Icon, { type IconName } from "./components/Icon.svelte";
  import Overview from "./views/Overview.svelte";
  import Scanner from "./views/Scanner.svelte";
  import Baselines from "./views/Baselines.svelte";
  import About from "./views/About.svelte";

  const win = getCurrentWindow();
  let maximized = $state(false);

  const nav: { id: View; label: string; icon: IconName }[] = [
    { id: "overview", label: "Overview", icon: "grid" },
    { id: "scan", label: "Scan", icon: "search" },
    { id: "baselines", label: "Baselines", icon: "database" },
    { id: "about", label: "About", icon: "info" },
  ];

  const last = $derived.by(() => {
    const r = scan.result;
    if (!r) return null;
    let danger = 0;
    let warn = 0;
    for (const f of r.reports) {
      if (f.level === "Danger") danger++;
      else if (f.level === "Warn") warn++;
    }
    return { total: r.reports.length, danger, warn };
  });

  onMount(() => {
    backdrop()
      .then((b) => document.documentElement.classList.toggle("mica", b === "mica"))
      .catch(() => {});
    loadQuickPaths();
    loadSystemInfo();
    refreshBaselines();
    let unlisten: (() => void) | undefined;
    const sync = async () => (maximized = await win.isMaximized());
    sync();
    win.onResized(sync).then((u) => (unlisten = u));
    return () => unlisten?.();
  });
</script>

<div class="shell">
  <header class="titlebar" data-tauri-drag-region>
    <svg class="app-icon" viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
      <path fill="var(--accent)" d="M12 2.5 4.5 5.3v5.6c0 4.7 3.2 9 7.5 10.6 4.3-1.6 7.5-5.9 7.5-10.6V5.3L12 2.5z" />
      <path fill="none" stroke="var(--on-accent)" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" d="m8.6 12.2 2.4 2.4 4.4-4.6" />
    </svg>
    <span class="app-title" data-tauri-drag-region>hashguard</span>

    <div class="caption-buttons">
      <button class="cap" onclick={() => win.minimize()} aria-label="Minimize">
        <svg viewBox="0 0 10 10" width="10" height="10"><path d="M0 5.5h10" stroke="currentColor" /></svg>
      </button>
      <button class="cap" onclick={() => win.toggleMaximize()} aria-label={maximized ? "Restore" : "Maximize"}>
        {#if maximized}
          <svg viewBox="0 0 10 10" width="10" height="10" fill="none" stroke="currentColor">
            <path d="M2.5 2.5V1.5a1 1 0 0 1 1-1h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-1" /><rect x=".5" y="2.5" width="7" height="7" rx="1" />
          </svg>
        {:else}
          <svg viewBox="0 0 10 10" width="10" height="10" fill="none" stroke="currentColor">
            <rect x=".5" y=".5" width="9" height="9" rx="1" />
          </svg>
        {/if}
      </button>
      <button class="cap close" onclick={() => win.close()} aria-label="Close">
        <svg viewBox="0 0 10 10" width="10" height="10"><path d="m.5.5 9 9m0-9-9 9" stroke="currentColor" /></svg>
      </button>
    </div>
  </header>

  <div class="body">
    <nav class="pane" aria-label="Sections">
      {#each nav as item}
        <button
          class="nav-item"
          class:selected={ui.view === item.id}
          aria-current={ui.view === item.id ? "page" : undefined}
          onclick={() => (ui.view = item.id)}
        >
          <Icon name={item.icon} size={16} />
          <span>{item.label}</span>
        </button>
      {/each}

      <div class="spacer"></div>

      <button class="last card" onclick={() => (ui.view = last ? "scan" : "overview")}>
        <span class="status-dot {last ? (last.danger ? 'danger' : last.warn ? 'warn' : 'ok') : 'idle'}"></span>
        <span class="last-text">
          <span class="body-strong">
            {#if !last}Not scanned yet
            {:else if last.danger}{fmtNum(last.danger)} dangerous
            {:else if last.warn}{fmtNum(last.warn)} to review
            {:else}All clear{/if}
          </span>
          <span class="caption num">
            {#if last}{fmtNum(last.total)} files · {timeAgo(scan.finishedAt)}{:else}Run a scan to get a status{/if}
          </span>
        </span>
      </button>

      <span class="version caption">hashguard 0.3.1</span>
    </nav>

    <main class="content">
      {#key ui.view}
        <div class="scroll appear">
          {#if ui.view === "overview"}
            <Overview />
          {:else if ui.view === "scan"}
            <Scanner />
          {:else if ui.view === "baselines"}
            <Baselines />
          {:else}
            <About />
          {/if}
        </div>
      {/key}
    </main>
  </div>
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .titlebar {
    display: flex;
    align-items: center;
    height: 48px;
    flex-shrink: 0;
    padding-left: 16px;
  }

  .app-icon {
    flex-shrink: 0;
  }

  .app-title {
    margin-left: 12px;
    font-size: 12px;
  }

  .caption-buttons {
    margin-left: auto;
    display: flex;
    align-self: flex-start;
    height: 32px;
  }

  .cap {
    width: 46px;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text);
  }

  .cap:hover {
    background: var(--subtle-hover);
  }

  .cap.close:hover {
    background: #ffffff;
    color: #000000;
  }

  .cap:focus-visible {
    border-radius: 0;
    outline-offset: -2px;
  }

  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 248px 1fr;
  }

  .pane {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px 8px 12px;
  }

  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    height: 36px;
    padding: 0 14px;
    border-radius: var(--r-ctl);
    text-align: left;
    color: var(--text);
    transition: background 0.08s;
  }

  .nav-item:hover {
    background: var(--subtle-hover);
  }

  .nav-item:active {
    background: var(--subtle-press);
    color: var(--text-2);
  }

  .nav-item.selected {
    background: var(--subtle-hover);
  }

  /* Fluent selection pill */
  .nav-item.selected::before {
    content: "";
    position: absolute;
    left: 0;
    top: 50%;
    width: 3px;
    height: 16px;
    margin-top: -8px;
    border-radius: 2px;
    background: var(--accent);
  }

  .last {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    text-align: left;
  }

  .last-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--text-3);
  }
  .status-dot.ok {
    background: var(--ok);
  }
  .status-dot.warn {
    background: var(--warn);
    box-shadow: 0 0 0 4px var(--warn-bg);
  }
  .status-dot.danger {
    background: var(--danger);
    box-shadow: 0 0 14px 2px rgba(255, 255, 255, 0.55);
  }

  .last:hover {
    background: var(--ctl-hover);
  }

  .spacer {
    flex: 1;
  }

  .version {
    padding: 10px 14px 0;
    color: var(--text-3);
  }

  .content {
    min-width: 0;
    min-height: 0;
    position: relative;
    background: var(--layer);
    border-top: 1px solid var(--layer-stroke);
    border-left: 1px solid var(--layer-stroke);
    border-top-left-radius: var(--r-card);
  }

  .scroll {
    position: absolute;
    inset: 0;
    overflow: auto;
  }
</style>
