<script lang="ts">
  import { ui, scan, system, baselines, quickScan, runVerify } from "../lib/app.svelte";
  import { pickFolder } from "../lib/api";
  import { fmtNum, formatDate, pct, pluralFiles, splitPath, timeAgo } from "../lib/format";
  import type { FileReport, Level } from "../lib/types";
  import Icon from "../components/Icon.svelte";
  import LevelPill from "../components/LevelPill.svelte";
  import ProgressRing from "../components/ProgressRing.svelte";

  const reports = $derived(scan.result?.reports ?? []);
  const total = $derived(reports.length);
  const counts = $derived.by(() => {
    const c: Record<Level, number> = { Ok: 0, Info: 0, Warn: 0, Danger: 0 };
    for (const r of reports) c[r.level]++;
    return c;
  });
  const state = $derived(
    scan.running ? "running" : !scan.result ? "idle" : counts.Danger ? "danger" : counts.Warn ? "warn" : "ok",
  );
  const cleanShare = $derived(total ? counts.Ok / total : 0);
  const findings = $derived(reports.filter((r) => r.level !== "Ok").slice(0, 6));
  const progress = $derived(scan.total ? scan.done / scan.total : 0);
  const primary = $derived(scan.quick[0]);

  // Four concentric rings, outermost = clean, innermost = danger: reading inward
  // goes deeper, like the levels on the site. Small counts are boosted so a single
  // dangerous file is still visible.
  const boost = (n: number) => (!total || !n ? 0 : Math.max(0.06, Math.min(1, Math.sqrt(n / total))));
  const rings = $derived([
    { r: 104, tone: "ok", frac: state === "running" ? progress : cleanShare },
    { r: 82, tone: "info", frac: boost(counts.Info) },
    { r: 60, tone: "warn", frac: boost(counts.Warn) },
    { r: 38, tone: "danger", frac: boost(counts.Danger) },
  ]);

  function openFinding(r: FileReport) {
    scan.selected = r;
    scan.filter = "all";
    ui.view = "scan";
  }

  function review(level: Level) {
    scan.filter = level;
    scan.selected = null;
    ui.view = "scan";
  }

  async function customScan() {
    const p = await pickFolder("Folder to scan");
    if (p) quickScan(p);
  }

  function verify(name: string) {
    ui.view = "baselines";
    runVerify(name);
  }
</script>

<div class="page">
  <h1 class="title">Overview</h1>

  <div class="grid">
    <!-- Status hero -->
    <section class="card hero {state}">
      <div class="meter">
        <svg class="rings" viewBox="0 0 224 224" aria-hidden="true">
          {#each rings as g}
            {@const c = 2 * Math.PI * g.r}
            <circle class="track" cx="112" cy="112" r={g.r} />
            <circle
              class="arc {g.tone}"
              cx="112"
              cy="112"
              r={g.r}
              stroke-dasharray="{c * g.frac} {c}"
            />
          {/each}
        </svg>
        <div class="meter-inner">
          {#if state === "running"}
            <span class="meter-value num">{Math.floor(progress * 100)}%</span>
            <span class="caption">scanned</span>
          {:else if state === "idle"}
            <Icon name="shield" size={40} />
          {:else}
            <span class="meter-value num">{pct(counts.Ok, total)}</span>
            <span class="caption">clean</span>
          {/if}
        </div>
      </div>

      <div class="hero-text">
        <span class="eyebrow caption">
          {#if state === "running"}Scan in progress{:else if state === "idle"}System status{:else}Last scan · {timeAgo(scan.finishedAt)}{/if}
        </span>
        <h2 class="hero-title">
          {#if state === "running"}Checking {fmtNum(scan.done)} of {fmtNum(scan.total || 0)} files
          {:else if state === "idle"}Your system hasn't been checked yet
          {:else if state === "danger"}{counts.Danger === 1 ? "1 dangerous file found" : `${fmtNum(counts.Danger)} dangerous files found`}
          {:else if state === "warn"}{fmtNum(counts.Warn)} {counts.Warn === 1 ? "file needs" : "files need"} a closer look
          {:else}No critical findings{/if}
        </h2>
        <p class="hero-sub">
          {#if state === "running"}{scan.path}
          {:else if state === "idle"}Verify the Authenticode signature and SHA-256 of every Windows binary, then flag anything that looks out of place.
          {:else}{scan.lastPath} · {pluralFiles(total)} · {fmtNum(counts.Info)} informational{/if}
        </p>

        <div class="hero-actions">
          {#if state === "running"}
            <button class="btn" onclick={() => (ui.view = "scan")}><ProgressRing size={16} />View progress</button>
          {:else if state === "idle"}
            {#if primary}
              <button class="btn accent" onclick={() => quickScan(primary.path)}>
                <Icon name="search" size={16} />Scan {primary.label}
              </button>
            {/if}
            <button class="btn" onclick={customScan}>Choose folder…</button>
          {:else}
            {#if counts.Danger || counts.Warn}
              <button class="btn accent" onclick={() => review(counts.Danger ? "Danger" : "Warn")}>
                Review findings<Icon name="chevronRight" size={14} />
              </button>
            {:else}
              <button class="btn accent" onclick={() => (ui.view = "scan")}>View results<Icon name="chevronRight" size={14} /></button>
            {/if}
            <button class="btn" onclick={() => quickScan(scan.lastPath)}>Scan again</button>
          {/if}
        </div>
      </div>

      {#if scan.result && !scan.running}
        <div class="hero-stats">
          <button class="hs" onclick={() => review("Danger")}>
            <span class="hs-value num c-danger">{fmtNum(counts.Danger)}</span>
            <span class="caption">Danger</span>
          </button>
          <button class="hs" onclick={() => review("Warn")}>
            <span class="hs-value num c-warn">{fmtNum(counts.Warn)}</span>
            <span class="caption">Warning</span>
          </button>
          <button class="hs" onclick={() => review("Info")}>
            <span class="hs-value num">{fmtNum(counts.Info)}</span>
            <span class="caption">Info</span>
          </button>
        </div>
      {/if}
    </section>

    <!-- System -->
    <section class="card system">
      <div class="card-head">
        <span class="body-strong">This PC</span>
      </div>
      <dl>
        <div class="kv"><dt class="caption">Operating system</dt><dd>{system.info ? `${system.info.os}` : "—"}</dd></div>
        <div class="kv"><dt class="caption">Build</dt><dd class="num">{system.info?.build || "—"}</dd></div>
        <div class="kv"><dt class="caption">Scan threads</dt><dd class="num">{system.info?.cores ?? "—"}</dd></div>
        <div class="kv"><dt class="caption">Signature engine</dt><dd>WinVerifyTrust + catalogs</dd></div>
        <div class="kv"><dt class="caption">Baselines saved</dt><dd class="num">{baselines.list.length}</dd></div>
      </dl>
    </section>

    <!-- Quick scan -->
    <section class="quick-wrap">
      <div class="section-head">
        <h2 class="section-title">Quick scan</h2>
      </div>
      <div class="quick">
        {#each scan.quick as q}
          <button class="card tile" onclick={() => quickScan(q.path)} disabled={scan.running}>
            <div class="glyph accent"><Icon name="folder" size={20} /></div>
            <span class="body-strong">{q.label}</span>
            <span class="caption mono tile-path" title={q.path}>{q.path}</span>
            <span class="tile-go caption">Scan<Icon name="chevronRight" size={12} /></span>
          </button>
        {/each}
        <button class="card tile dashed" onclick={customScan} disabled={scan.running}>
          <div class="glyph"><Icon name="search" size={20} /></div>
          <span class="body-strong">Custom folder</span>
          <span class="caption">Pick any folder on disk</span>
          <span class="tile-go caption">Browse<Icon name="chevronRight" size={12} /></span>
        </button>
      </div>
    </section>

    <!-- Findings -->
    <section class="card findings">
      <div class="card-head">
        <span class="body-strong">Recent findings</span>
        {#if scan.result}
          <button class="btn subtle link" onclick={() => (ui.view = "scan")}>All results<Icon name="chevronRight" size={12} /></button>
        {/if}
      </div>
      {#if findings.length}
        <div class="rows">
          {#each findings as r (r.path)}
            {@const p = splitPath(r.path)}
            <button class="row" onclick={() => openFinding(r)}>
              <LevelPill level={r.level} label={false} />
              <span class="row-text">
                <span class="row-name">{p.name}</span>
                <span class="caption row-sub">{r.findings[0]}</span>
              </span>
              <Icon name="chevronRight" size={14} />
            </button>
          {/each}
        </div>
      {:else}
        <div class="empty">
          <div class="glyph {scan.result ? 'ok' : ''}"><Icon name={scan.result ? "shieldCheck" : "list"} size={20} /></div>
          <span class="caption">
            {scan.result ? "Nothing suspicious in the last scan." : "Findings from your last scan will appear here."}
          </span>
        </div>
      {/if}
    </section>

    <!-- Baselines -->
    <section class="card bl">
      <div class="card-head">
        <span class="body-strong">Baselines</span>
        <button class="btn subtle link" onclick={() => (ui.view = "baselines")}>Manage<Icon name="chevronRight" size={12} /></button>
      </div>
      {#if baselines.list.length}
        <div class="rows">
          {#each baselines.list.slice(0, 4) as b (b.name)}
            <div class="row static">
              <div class="glyph small"><Icon name="database" size={16} /></div>
              <span class="row-text">
                <span class="row-name">{b.name}</span>
                <span class="caption row-sub num">{pluralFiles(b.files)} · {formatDate(b.created_unix)}</span>
              </span>
              <button class="btn" onclick={() => verify(b.name)} disabled={!!baselines.busy}>Verify</button>
            </div>
          {/each}
        </div>
      {:else}
        <div class="empty">
          <div class="glyph"><Icon name="database" size={20} /></div>
          <span class="caption">Snapshot a clean folder to detect any change later.</span>
          <button class="btn" onclick={() => (ui.view = "baselines")}>Create baseline</button>
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .page {
    padding: 28px 36px 36px;
  }

  .title {
    margin-bottom: 20px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 16px;
  }

  /* Hero */
  .hero {
    grid-column: span 8;
    position: relative;
    display: flex;
    align-items: center;
    gap: 32px;
    padding: 28px 32px;
    overflow: hidden;
    min-height: 220px;
  }

  .hero::before {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: radial-gradient(120% 140% at 0% 0%, var(--tint) 0%, transparent 55%);
    opacity: 0.9;
  }

  /* The ring measures the clean share, so it is always the "good" color once a
     scan exists; the status itself is carried by the title and the tint. */
  .hero.idle,
  .hero.running {
    --tint: var(--info-bg);
    --ring: var(--accent);
  }
  .hero.ok {
    --tint: var(--ok-bg);
    --ring: var(--ok);
  }
  .hero.warn {
    --tint: var(--warn-bg);
    --ring: var(--ok);
  }
  .hero.danger {
    --tint: var(--danger-bg);
    --ring: var(--ok);
  }

  .meter {
    position: relative;
    flex-shrink: 0;
    width: 200px;
    height: 200px;
  }

  .rings {
    position: absolute;
    inset: 0;
    transform: rotate(-90deg);
  }

  .rings circle {
    fill: none;
    stroke-width: 6;
  }

  .rings .track {
    stroke: var(--track);
  }

  .rings .arc {
    stroke-linecap: round;
    transition: stroke-dasharray 0.5s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .arc.ok {
    stroke: var(--ok);
  }
  .arc.info {
    stroke: var(--info);
  }
  .arc.warn {
    stroke: var(--warn);
  }
  .arc.danger {
    stroke: var(--danger);
    filter: drop-shadow(0 0 5px rgba(255, 255, 255, 0.7));
  }

  .meter-inner {
    position: absolute;
    inset: 62px;
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--ring);
  }

  .meter-value {
    font-family: var(--font-display);
    font-size: 20px;
    line-height: 24px;
    font-weight: 600;
    color: var(--text);
  }

  .hero-text {
    position: relative;
    flex: 1;
    min-width: 0;
  }

  .eyebrow {
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 600;
  }

  .hero-title {
    margin-top: 6px;
    font-family: var(--font-display);
    font-size: 26px;
    line-height: 34px;
    font-weight: 600;
  }

  .hero-sub {
    margin: 6px 0 0;
    color: var(--text-2);
    max-width: 560px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hero-actions {
    display: flex;
    gap: 8px;
    margin-top: 20px;
  }

  .hero-actions .btn.accent {
    min-width: 140px;
  }

  .hero-stats {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-left: 24px;
    border-left: 1px solid var(--divider);
  }

  .hs {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 4px 10px;
    border-radius: var(--r-ctl);
    text-align: left;
  }

  .hs:hover {
    background: var(--subtle-hover);
  }

  .hs-value {
    min-width: 44px;
    font-family: var(--font-display);
    font-size: 24px;
    line-height: 30px;
    font-weight: 600;
  }

  /* System */
  .system {
    grid-column: span 4;
    padding: 18px 20px;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 28px;
    margin-bottom: 8px;
  }

  dl {
    margin: 0;
  }

  .kv {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 0;
    border-top: 1px solid var(--divider);
  }

  .kv dt {
    color: var(--text-2);
    font-size: 13px;
  }

  .kv dd {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    text-align: right;
  }

  /* Quick scan */
  .quick-wrap {
    grid-column: span 12;
  }

  .section-head {
    margin: 8px 0 10px;
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 20px;
    line-height: 28px;
  }

  .quick {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 12px;
  }

  .tile {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 16px;
    text-align: left;
    transition: background 0.1s, border-color 0.1s;
  }

  .tile:hover:not(:disabled) {
    background: var(--ctl-hover);
    border-color: color-mix(in srgb, var(--accent) 40%, var(--stroke));
  }

  .tile .glyph {
    margin-bottom: 10px;
  }

  .tile-path {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-3);
  }

  .tile-go {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    margin-top: 10px;
    color: var(--accent);
    font-weight: 600;
  }

  .tile.dashed {
    border-style: dashed;
    background: transparent;
    box-shadow: none;
  }

  /* Findings + baselines */
  .findings {
    grid-column: span 7;
    padding: 18px 20px;
    min-height: 300px;
  }

  .bl {
    grid-column: span 5;
    padding: 18px 20px;
  }

  .link {
    height: 28px;
    padding: 0 8px;
    color: var(--accent);
    gap: 2px;
  }

  .rows {
    display: flex;
    flex-direction: column;
    margin: 0 -8px;
  }

  .row {
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    min-height: 52px;
    padding: 6px 8px;
    border-radius: var(--r-ctl);
    text-align: left;
    color: var(--text-3);
  }

  .row:not(.static):hover {
    background: var(--subtle-hover);
  }

  .row.static {
    grid-template-columns: 32px minmax(0, 1fr) auto;
  }

  .row + .row {
    border-top: 1px solid var(--divider);
  }

  .row-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    color: var(--text);
  }

  .row-name,
  .row-sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-sub::first-letter {
    text-transform: uppercase;
  }

  .glyph.small {
    width: 32px;
    height: 32px;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 40px 16px;
    text-align: center;
  }

  @media (max-width: 1280px) {
    .hero,
    .system,
    .findings,
    .bl {
      grid-column: span 12;
    }
  }
</style>
