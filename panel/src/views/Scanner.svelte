<script lang="ts">
  import { scan, runScan, stopScan, type Filter } from "../lib/app.svelte";
  import { pickFolder, pickFile, saveTextFile } from "../lib/api";
  import { fmtNum, formatDuration, levelLabel, pct, pluralFiles, sigLabel, splitPath } from "../lib/format";
  import type { Level, SigStatus } from "../lib/types";
  import Icon from "../components/Icon.svelte";
  import LevelPill, { levelTone } from "../components/LevelPill.svelte";
  import InfoBar from "../components/InfoBar.svelte";
  import StatTile from "../components/StatTile.svelte";
  import ProgressRing from "../components/ProgressRing.svelte";
  import FileDetails from "../components/FileDetails.svelte";

  const PAGE = 200;
  let limit = $state(PAGE);
  let showBlocklist = $state(false);
  let notice = $state("");

  const reports = $derived(scan.result?.reports ?? []);
  const total = $derived(reports.length);
  const counts = $derived.by(() => {
    const c: Record<Level, number> = { Ok: 0, Info: 0, Warn: 0, Danger: 0 };
    for (const r of reports) c[r.level]++;
    return c;
  });
  const sig = $derived.by(() => {
    const c: Record<SigStatus, number> = { Valid: 0, Unsigned: 0, Invalid: 0, Error: 0 };
    let catalog = 0;
    for (const r of reports) {
      c[r.signature.status]++;
      if (r.signature.status === "Valid" && r.signature.via_catalog) catalog++;
    }
    return { ...c, catalog };
  });
  const sigRows = $derived(
    [
      { label: "Signed, embedded", n: sig.Valid - sig.catalog },
      { label: "Signed, via catalog", n: sig.catalog },
      { label: "Unsigned", n: sig.Unsigned },
      { label: "Invalid signature", n: sig.Invalid },
      { label: "Not checked", n: sig.Error },
    ].sort((a, b) => b.n - a.n),
  );
  const sigMax = $derived(Math.max(1, ...sigRows.map((r) => r.n)));
  const publishers = $derived.by(() => {
    const m = new Map<string, number>();
    for (const r of reports) {
      if (r.signature.status === "Valid" && r.signature.signer) {
        m.set(r.signature.signer, (m.get(r.signature.signer) ?? 0) + 1);
      }
    }
    return [...m.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5);
  });
  const pubMax = $derived(publishers[0]?.[1] ?? 1);

  const filtered = $derived.by(() => {
    const q = scan.query.trim().toLowerCase();
    return reports.filter(
      (r) =>
        (scan.filter === "all" || r.level === scan.filter) &&
        (!q ||
          r.path.toLowerCase().includes(q) ||
          (r.signature.signer ?? "").toLowerCase().includes(q) ||
          r.sha256.startsWith(q)),
    );
  });
  const shown = $derived(filtered.slice(0, limit));
  const progress = $derived(scan.total ? (scan.done / scan.total) * 100 : 0);
  const verdict = $derived(counts.Danger ? "danger" : counts.Warn ? "warn" : "ok");

  $effect(() => {
    scan.filter;
    scan.query;
    limit = PAGE;
  });

  // Worst first: problems lead the bar, the legend and the filter.
  const levels: Level[] = ["Danger", "Warn", "Info", "Ok"];
  const filters: { id: Filter; label: string }[] = [
    { id: "all", label: "All" },
    ...levels.map((l) => ({ id: l as Filter, label: levelLabel[l] })),
  ];

  async function chooseFolder() {
    const p = await pickFolder("Folder to scan");
    if (p) scan.path = p;
  }

  async function chooseBlocklist() {
    const p = await pickFile("List of known-bad hashes");
    if (p) scan.blocklist = p;
  }

  async function exportJson() {
    if (!scan.result) return;
    try {
      const saved = await saveTextFile("hashguard-report.json", JSON.stringify(scan.result.reports, null, 2));
      if (saved) notice = "Saved";
    } catch {
      notice = "Failed";
    }
    setTimeout(() => (notice = ""), 2500);
  }
</script>

<div class="page">
  <h1 class="title">Scan</h1>

  <!-- Target -->
  <section class="card target">
    <div class="target-row">
      <div class="glyph accent"><Icon name="folder" size={20} /></div>
      <input
        class="textbox mono"
        placeholder="C:\Windows\System32"
        bind:value={scan.path}
        onkeydown={(e) => e.key === "Enter" && runScan()}
        disabled={scan.running}
        spellcheck="false"
        aria-label="Folder to scan"
      />
      <button class="btn" onclick={chooseFolder} disabled={scan.running}>Browse…</button>
      {#if scan.running}
        <button class="btn" onclick={stopScan}><Icon name="stop" size={16} />Stop</button>
      {:else}
        <button class="btn accent" onclick={runScan} disabled={!scan.path.trim()}>
          <Icon name="search" size={16} />Scan
        </button>
      {/if}
    </div>

    {#if scan.running}
      <div class="progress">
        <ProgressRing />
        <span class="num">
          {#if scan.total}Checked {fmtNum(scan.done)} of {fmtNum(scan.total)} files{:else}Looking for executables…{/if}
        </span>
        <span class="caption num pct">{scan.total ? `${Math.floor(progress)}%` : ""}</span>
      </div>
      <div class="bar"><div class="bar-fill" style="width: {progress}%"></div></div>
    {:else}
      <div class="quick">
        {#each scan.quick as q}
          <button class="pill" class:on={scan.path === q.path} onclick={() => (scan.path = q.path)} title={q.path}>
            {q.label}
          </button>
        {/each}
        <button class="pill ghost" onclick={() => (showBlocklist = !showBlocklist)} aria-expanded={showBlocklist}>
          <Icon name="list" size={14} />Blocklist{scan.blocklist ? " · on" : ""}
        </button>
      </div>
    {/if}

    {#if showBlocklist && !scan.running}
      <div class="target-row blocklist appear">
        <input
          class="textbox mono"
          placeholder="Text file with known-bad SHA-256 hashes, one per line"
          bind:value={scan.blocklist}
          spellcheck="false"
          aria-label="Blocklist file"
        />
        <button class="btn" onclick={chooseBlocklist}>Choose…</button>
        {#if scan.blocklist}<button class="btn subtle" onclick={() => (scan.blocklist = "")}>Remove</button>{/if}
      </div>
    {/if}
  </section>

  {#if scan.error}
    <InfoBar severity="danger" title="The scan could not run">{scan.error}</InfoBar>
  {/if}

  {#if scan.result}
    {@const res = scan.result}
    <InfoBar
      severity={verdict}
      title={counts.Danger
        ? `Dangerous files found: ${fmtNum(counts.Danger)}`
        : counts.Warn
          ? `Files to review: ${fmtNum(counts.Warn)}`
          : "No critical findings"}
    >
      {pluralFiles(total)} in {formatDuration(res.elapsed_ms)}{res.cancelled
        ? " · stopped early, results are partial"
        : ""}{res.failures.length ? ` · ${fmtNum(res.failures.length)} unreadable` : ""}
    </InfoBar>

    <div class="kpis">
      <StatTile icon="files" tone="accent" label="Files checked" value={fmtNum(total)} sub={`in ${formatDuration(res.elapsed_ms)}`} />
      <StatTile icon="shieldError" tone={counts.Danger ? "danger" : "neutral"} label="Danger" value={fmtNum(counts.Danger)} sub={counts.Danger ? "act on these first" : "none found"} />
      <StatTile icon="warning" tone={counts.Warn ? "warn" : "neutral"} label="Warning" value={fmtNum(counts.Warn)} sub={counts.Warn ? "worth a look" : "none found"} />
      <StatTile icon="shieldCheck" tone="ok" label="Valid signatures" value={pct(sig.Valid, total)} sub={`${fmtNum(sig.Unsigned)} unsigned`} />
    </div>

    <div class="charts">
      <section class="card chart">
        <div class="chart-head">
          <span class="body-strong">Severity</span>
          <span class="caption">How files were classified</span>
        </div>
        <div class="segbar" role="img" aria-label="Files by severity">
          {#each levels as l}
            {#if counts[l]}
              <div class="seg {levelTone[l]}" style="flex-grow: {counts[l]}" title="{levelLabel[l]}: {fmtNum(counts[l])}"></div>
            {/if}
          {/each}
        </div>
        <div class="legend">
          {#each levels as l}
            <button class="legend-row" class:on={scan.filter === l} class:zero={!counts[l]} onclick={() => (scan.filter = scan.filter === l ? "all" : l)}>
              <LevelPill level={l} />
              <span class="num count">{fmtNum(counts[l])}</span>
              <span class="caption num share">{pct(counts[l], total)}</span>
            </button>
          {/each}
        </div>
      </section>

      <section class="card chart">
        <div class="chart-head">
          <span class="body-strong">Signatures</span>
          <span class="caption">How each file proved where it came from</span>
        </div>
        <div class="meters">
          {#each sigRows as r}
            <div class="meter-row" class:zero={!r.n} title="{r.label}: {fmtNum(r.n)} ({pct(r.n, total)})">
              <span class="m-name">{r.label}</span>
              <div class="m-track"><div class="m-fill" style="width: {(r.n / sigMax) * 100}%"></div></div>
              <span class="num m-count">{fmtNum(r.n)}</span>
            </div>
          {/each}
        </div>
      </section>

      <section class="card chart">
        <div class="chart-head">
          <span class="body-strong">Top publishers</span>
          <span class="caption">Who signed the files with valid signatures</span>
        </div>
        {#if publishers.length}
          <div class="meters">
            {#each publishers as [name, n]}
              <div class="meter-row" title="{name}: {fmtNum(n)} ({pct(n, total)})">
                <span class="m-name">{name}</span>
                <div class="m-track"><div class="m-fill" style="width: {(n / pubMax) * 100}%"></div></div>
                <span class="num m-count">{fmtNum(n)}</span>
              </div>
            {/each}
          </div>
        {:else}
          <div class="caption empty-chart">No files with a valid signature</div>
        {/if}
      </section>
    </div>

    <section class="card files" class:split={!!scan.selected}>
      <div class="files-main">
        <div class="files-head">
          <div class="selector" role="tablist" aria-label="Filter by severity">
            {#each filters as f}
              <button role="tab" aria-selected={scan.filter === f.id} class="sel-item" class:on={scan.filter === f.id} onclick={() => (scan.filter = f.id)}>
                {f.label}
                <span class="caption num">{fmtNum(f.id === "all" ? total : counts[f.id as Level])}</span>
              </button>
            {/each}
          </div>
          <div class="search">
            <input class="textbox" placeholder="Search path, publisher or hash" bind:value={scan.query} spellcheck="false" aria-label="Search" />
            <span class="search-ico"><Icon name="search" size={14} /></span>
          </div>
          <button class="btn subtle" onclick={exportJson} title="Export as JSON">
            <Icon name={notice ? "check" : "download"} size={16} />{notice || "Export"}
          </button>
        </div>

        <div class="list-head caption">
          <span></span><span>File</span><span>Signature</span><span>Publisher</span><span></span>
        </div>

        <div class="list" role="list">
          {#each shown as r (r.path)}
            {@const p = splitPath(r.path)}
            <button class="item" class:sel={scan.selected?.path === r.path} onclick={() => (scan.selected = scan.selected?.path === r.path ? null : r)}>
              <LevelPill level={r.level} label={false} />
              <span class="item-text">
                <span class="item-name">{p.name}</span>
                {#if r.findings.length}
                  <span class="caption item-sub {r.level === 'Info' ? '' : `c-${levelTone[r.level]}`}">{r.findings[0]}</span>
                {:else}
                  <span class="caption item-sub mono">{p.dir}</span>
                {/if}
              </span>
              <span class="caption item-sig">{sigLabel[r.signature.status]}{r.signature.via_catalog ? " · catalog" : ""}</span>
              <span class="caption item-pub">{r.signature.signer ?? "—"}</span>
              <span class="item-chev"><Icon name="chevronRight" size={14} /></span>
            </button>
          {:else}
            <div class="caption empty">{total ? "Nothing matches this filter" : "No executable files found"}</div>
          {/each}
          {#if filtered.length > shown.length}
            <button class="btn more" onclick={() => (limit += PAGE)}>
              Show {fmtNum(Math.min(PAGE, filtered.length - shown.length))} more
            </button>
          {/if}
        </div>
      </div>

      {#if scan.selected}
        <FileDetails file={scan.selected} onclose={() => (scan.selected = null)} />
      {/if}
    </section>
  {:else if !scan.running && !scan.error}
    <div class="card hint appear">
      <div class="glyph accent"><Icon name="shieldCheck" size={20} /></div>
      <div>
        <div class="body-strong">Pick a folder and press Scan</div>
        <div class="caption">
          Every .exe, .dll and .sys inside is hashed with SHA-256, its Authenticode signature is
          verified (including Windows catalogs), and heuristics flag anything that looks out of place.
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .page {
    padding: 28px 36px 36px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .title {
    margin-bottom: 4px;
  }

  .target {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .target-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .target-row .glyph {
    width: 32px;
    height: 32px;
    margin-right: 4px;
  }

  .target-row .btn.accent {
    min-width: 120px;
  }

  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding-left: 44px;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: 14px;
    background: var(--ctl);
    border: 1px solid var(--ctl-stroke);
    font-size: 13px;
  }

  .pill:hover {
    background: var(--ctl-hover);
  }

  .pill.on {
    background: var(--accent);
    border-color: transparent;
    color: var(--on-accent);
  }

  .pill.ghost {
    margin-left: auto;
    background: transparent;
    border-style: dashed;
    color: var(--text-2);
  }

  .blocklist {
    padding-left: 44px;
  }

  .progress {
    display: flex;
    align-items: center;
    gap: 12px;
    padding-left: 44px;
  }

  .pct {
    margin-left: auto;
  }

  .bar {
    position: relative;
    height: 3px;
    margin: -6px 0 0 44px;
  }

  .bar::before {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 1px;
    height: 1px;
    background: var(--text-3);
    opacity: 0.6;
  }

  .bar-fill {
    position: relative;
    height: 3px;
    border-radius: 2px;
    background: var(--accent);
    transition: width 0.2s linear;
  }

  .kpis {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }

  .charts {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }

  .chart {
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .chart-head {
    display: flex;
    flex-direction: column;
  }

  .segbar {
    display: flex;
    gap: 2px;
    height: 8px;
  }

  .seg {
    min-width: 4px;
    border-radius: 4px;
  }
  .seg.ok {
    background: var(--ok);
  }
  .seg.info {
    background: var(--info);
  }
  .seg.warn {
    background: var(--warn);
  }
  .seg.danger {
    background: var(--danger);
  }

  .legend {
    display: flex;
    flex-direction: column;
    margin: 0 -8px;
  }

  .legend-row {
    display: grid;
    grid-template-columns: 1fr 64px 56px;
    align-items: center;
    height: 32px;
    padding: 0 8px;
    border-radius: var(--r-ctl);
    text-align: left;
  }

  .legend-row:hover,
  .legend-row.on {
    background: var(--subtle-hover);
  }

  .legend-row.zero,
  .meter-row.zero {
    opacity: 0.5;
  }

  .count {
    text-align: right;
    font-weight: 600;
  }

  .share {
    text-align: right;
    color: var(--text-3);
  }

  .meters {
    display: flex;
    flex-direction: column;
    gap: 13px;
  }

  .meter-row {
    display: grid;
    grid-template-columns: minmax(0, 140px) 1fr 52px;
    align-items: center;
    gap: 12px;
  }

  .m-name {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .m-track {
    height: 8px;
    border-radius: 4px;
    background: var(--track);
    overflow: hidden;
  }

  .m-fill {
    height: 100%;
    min-width: 4px;
    border-radius: 4px;
    background: var(--accent);
  }

  .m-count {
    text-align: right;
    font-weight: 600;
  }

  .empty-chart {
    padding: 20px 0;
    color: var(--text-3);
  }

  /* Files */
  .files {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    height: max(560px, calc(100vh - 150px));
    overflow: hidden;
  }

  .files.split {
    grid-template-columns: minmax(0, 1fr) 360px;
  }

  .files-main {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .files-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    padding: 8px 12px 8px 8px;
    border-bottom: 1px solid var(--divider);
  }

  .selector {
    display: flex;
    gap: 2px;
    margin-right: auto;
  }

  .sel-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 10px;
    border-radius: var(--r-ctl);
    color: var(--text-2);
    white-space: nowrap;
  }

  .sel-item:hover {
    background: var(--subtle-hover);
    color: var(--text);
  }

  .sel-item.on {
    color: var(--text);
    font-weight: 600;
  }

  .sel-item.on::after {
    content: "";
    position: absolute;
    left: 50%;
    bottom: 2px;
    width: 16px;
    height: 3px;
    margin-left: -8px;
    border-radius: 2px;
    background: var(--accent);
  }

  .search {
    position: relative;
    flex: 1 1 200px;
    max-width: 320px;
  }

  .search .textbox {
    padding-right: 32px;
  }

  .search-ico {
    position: absolute;
    right: 10px;
    top: 9px;
    color: var(--text-2);
    pointer-events: none;
  }

  .list-head,
  .item {
    display: grid;
    grid-template-columns: 20px minmax(0, 2fr) minmax(0, 1fr) minmax(0, 1fr) 14px;
    align-items: center;
    gap: 16px;
  }

  .list-head {
    padding: 8px 16px;
    color: var(--text-3);
    border-bottom: 1px solid var(--divider);
  }

  .list {
    flex: 1;
    overflow: auto;
    padding: 4px;
  }

  .item {
    position: relative;
    width: 100%;
    min-height: 48px;
    padding: 6px 12px;
    border-radius: var(--r-ctl);
    text-align: left;
  }

  .item:hover,
  .item.sel {
    background: var(--subtle-hover);
  }

  .item.sel::before {
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

  .item-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .item-name,
  .item-sub,
  .item-sig,
  .item-pub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-sub::first-letter {
    text-transform: uppercase;
  }

  .item-sub.mono {
    color: var(--text-3);
    font-size: 11.5px;
  }

  .item-chev {
    display: flex;
    color: var(--text-3);
  }

  .split .list-head,
  .split .item {
    grid-template-columns: 20px minmax(0, 1fr) minmax(0, 120px) 14px;
  }

  .split .item-pub,
  .split .list-head span:nth-child(4) {
    display: none;
  }

  .empty {
    padding: 48px 0;
    text-align: center;
  }

  .more {
    display: flex;
    margin: 8px auto;
  }

  .hint {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 18px;
  }

  @media (max-width: 1280px) {
    .charts {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .kpis {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
