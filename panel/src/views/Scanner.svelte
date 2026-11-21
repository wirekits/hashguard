<script lang="ts">
  import { scan, baselines, ui, runScan, stopScan, type Filter } from "../lib/app.svelte";
  import { pickFolder, pickFile, saveTextFile } from "../lib/api";
  import {
    fmtNum,
    formatDuration,
    levelLabel,
    pluralFiles,
    sigLabel,
    splitPath,
    tFinding,
  } from "../lib/format";
  import type { Level } from "../lib/types";
  import Icon from "../components/Icon.svelte";
  import LevelPill, { levelTone } from "../components/LevelPill.svelte";
  import InfoBar from "../components/InfoBar.svelte";
  import StatTile from "../components/StatTile.svelte";
  import ProgressRing from "../components/ProgressRing.svelte";
  import FileDetails from "../components/FileDetails.svelte";

  const PAGE = 150;
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
    let valid = 0;
    let unsigned = 0;
    let invalid = 0;
    for (const r of reports) {
      if (r.signature.status === "Valid") valid++;
      else if (r.signature.status === "Unsigned") unsigned++;
      else if (r.signature.status === "Invalid") invalid++;
    }
    return { valid, unsigned, invalid };
  });
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
  const pct = $derived(scan.total ? (scan.done / scan.total) * 100 : 0);
  const verdict = $derived(counts.Danger ? "danger" : counts.Warn ? "warn" : "ok");

  $effect(() => {
    scan.filter;
    scan.query;
    limit = PAGE;
  });

  // Worst first: problems lead the bar, the legend and the filter.
  const levels: Level[] = ["Danger", "Warn", "Info", "Ok"];
  const filters: { id: Filter; label: string }[] = [
    { id: "all", label: "Все" },
    ...levels.map((l) => ({ id: l as Filter, label: levelLabel[l] })),
  ];

  const share = (n: number) =>
    total ? `${((n / total) * 100).toLocaleString("ru-RU", { maximumFractionDigits: 1 })} %` : "0 %";

  async function chooseFolder() {
    const p = await pickFolder("Папка для проверки");
    if (p) scan.path = p;
  }

  async function chooseBlocklist() {
    const p = await pickFile("Список известных вредоносных хешей");
    if (p) scan.blocklist = p;
  }

  async function exportJson() {
    if (!scan.result) return;
    try {
      const saved = await saveTextFile("hashguard-report.json", JSON.stringify(scan.result.reports, null, 2));
      if (saved) notice = "Сохранено";
    } catch (e) {
      notice = "Ошибка";
    }
    setTimeout(() => (notice = ""), 2500);
  }
</script>

<div class="page">
  <h1 class="title">Сканер</h1>

  <!-- Target -->
  <section class="card target">
    <div class="row-head">
      <div class="glyph accent"><Icon name="folder" size={20} /></div>
      <div class="head-text">
        <span>Папка для проверки</span>
        <span class="caption">Все .exe, .dll и .sys внутри, включая вложенные папки</span>
      </div>
    </div>

    <div class="target-row">
      <input
        class="textbox mono"
        placeholder="C:\Windows\System32"
        bind:value={scan.path}
        onkeydown={(e) => e.key === "Enter" && runScan()}
        disabled={scan.running}
        spellcheck="false"
        aria-label="Путь для сканирования"
      />
      <button class="btn" onclick={chooseFolder} disabled={scan.running}>Обзор…</button>
      {#if scan.running}
        <button class="btn" onclick={stopScan}><Icon name="stop" size={16} />Остановить</button>
      {:else}
        <button class="btn accent" onclick={runScan} disabled={!scan.path.trim()}>
          <Icon name="search" size={16} />Сканировать
        </button>
      {/if}
    </div>

    {#if scan.running}
      <div class="progress">
        <ProgressRing />
        <span class="num">
          {#if scan.total}Проверено {fmtNum(scan.done)} из {fmtNum(scan.total)}{:else}Поиск исполняемых файлов…{/if}
        </span>
        <span class="caption num pct">{scan.total ? `${Math.floor(pct)} %` : ""}</span>
      </div>
      <div class="bar"><div class="bar-fill" style="width: {pct}%"></div></div>
    {:else}
      <div class="quick">
        {#each scan.quick as q}
          <button class="pill" class:on={scan.path === q.path} onclick={() => (scan.path = q.path)} title={q.path}>
            {q.label}
          </button>
        {/each}
      </div>
    {/if}

    <button class="expander" onclick={() => (showBlocklist = !showBlocklist)} aria-expanded={showBlocklist}>
      <span>Список известных вредоносных хешей</span>
      <span class="caption">{scan.blocklist ? "подключён" : "необязательно"}</span>
      <span class="chev" class:open={showBlocklist}><Icon name="chevronDown" size={14} /></span>
    </button>
    {#if showBlocklist}
      <div class="target-row blocklist">
        <input
          class="textbox mono"
          placeholder="Текстовый файл, по одному SHA-256 в строке"
          bind:value={scan.blocklist}
          disabled={scan.running}
          spellcheck="false"
          aria-label="Файл со списком хешей"
        />
        <button class="btn" onclick={chooseBlocklist} disabled={scan.running}>Выбрать…</button>
      </div>
    {/if}
  </section>

  {#if scan.error}
    <InfoBar severity="danger" title="Не удалось выполнить проверку">{scan.error}</InfoBar>
  {/if}

  {#if scan.result}
    {@const res = scan.result}
    <div class="dash appear">
      <InfoBar
        severity={verdict}
        title={counts.Danger
          ? `Найдены опасные файлы: ${fmtNum(counts.Danger)}`
          : counts.Warn
            ? `Есть подозрительные файлы: ${fmtNum(counts.Warn)}`
            : "Критичных находок нет"}
      >
        {pluralFiles(total)} за {formatDuration(res.elapsed_ms)}{res.cancelled
          ? " · проверка остановлена, результат неполный"
          : ""}{res.failures.length ? ` · не удалось прочитать: ${fmtNum(res.failures.length)}` : ""}
      </InfoBar>

      <div class="kpis">
        <StatTile icon="files" tone="accent" label="Проверено файлов" value={fmtNum(total)} sub={`за ${formatDuration(res.elapsed_ms)}`} />
        <StatTile
          icon="shieldError"
          tone={counts.Danger ? "danger" : "neutral"}
          label="Опасно"
          value={fmtNum(counts.Danger)}
          sub={counts.Danger ? "нужно разобраться" : "не найдено"}
        />
        <StatTile
          icon="warning"
          tone={counts.Warn ? "warn" : "neutral"}
          label="Подозрительно"
          value={fmtNum(counts.Warn)}
          sub={counts.Warn ? "стоит посмотреть" : "не найдено"}
        />
        <StatTile
          icon="shieldCheck"
          tone="ok"
          label="Подпись в порядке"
          value={share(sig.valid)}
          sub={`${fmtNum(sig.unsigned)} без подписи`}
        />
      </div>

      <div class="charts">
        <section class="card chart">
          <div class="chart-head">
            <span class="body-strong">Уровни</span>
            <span class="caption">Распределение файлов по результату</span>
          </div>
          <div class="segbar" role="img" aria-label="Распределение по уровням">
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
                <span class="caption num share">{share(counts[l])}</span>
              </button>
            {/each}
          </div>
        </section>

        <section class="card chart">
          <div class="chart-head">
            <span class="body-strong">Издатели</span>
            <span class="caption">Кем подписаны проверенные файлы</span>
          </div>
          {#if publishers.length}
            <div class="pubs">
              {#each publishers as [name, n]}
                <div class="pub" title="{name}: {fmtNum(n)} ({share(n)})">
                  <span class="pub-name">{name}</span>
                  <div class="pub-track"><div class="pub-fill" style="width: {(n / pubMax) * 100}%"></div></div>
                  <span class="num pub-count">{fmtNum(n)}</span>
                </div>
              {/each}
            </div>
          {:else}
            <div class="caption empty-chart">Нет файлов с действительной подписью</div>
          {/if}
          <div class="caption foot num">
            Без подписи: {fmtNum(sig.unsigned)} · Подпись недействительна: {fmtNum(sig.invalid)}
          </div>
        </section>
      </div>

      <section class="card files" class:split={!!scan.selected}>
        <div class="files-main">
          <div class="files-head">
            <div class="selector" role="tablist" aria-label="Фильтр по уровню">
              {#each filters as f}
                <button role="tab" aria-selected={scan.filter === f.id} class="sel-item" class:on={scan.filter === f.id} onclick={() => (scan.filter = f.id)}>
                  {f.label}
                  <span class="caption num">{fmtNum(f.id === "all" ? total : counts[f.id as Level])}</span>
                </button>
              {/each}
            </div>
            <div class="search">
              <input class="textbox" placeholder="Поиск по пути, издателю, хешу" bind:value={scan.query} spellcheck="false" aria-label="Поиск" />
              <span class="search-ico"><Icon name="search" size={14} /></span>
            </div>
            <button class="btn subtle" onclick={exportJson} title="Экспорт в JSON">
              <Icon name={notice ? "check" : "download"} size={16} />{notice || "Экспорт"}
            </button>
          </div>

          <div class="list" role="list">
            {#each shown as r (r.path)}
              {@const p = splitPath(r.path)}
              <button class="item" class:sel={scan.selected?.path === r.path} onclick={() => (scan.selected = scan.selected?.path === r.path ? null : r)}>
                <LevelPill level={r.level} label={false} />
                <span class="item-text">
                  <span class="item-name">{p.name}</span>
                  {#if r.findings.length}
                    <span class="caption item-sub {r.level === 'Info' ? '' : `c-${levelTone[r.level]}`}">{tFinding(r.findings[0])}</span>
                  {:else}
                    <span class="caption item-sub mono">{p.dir}</span>
                  {/if}
                </span>
                <span class="caption item-sig">
                  {r.signature.status === "Valid" ? (r.signature.signer ?? sigLabel.Valid) : sigLabel[r.signature.status]}
                </span>
                <span class="item-chev"><Icon name="chevronRight" size={14} /></span>
              </button>
            {:else}
              <div class="caption empty">{total ? "Под этот фильтр ничего не подходит" : "Исполняемые файлы не найдены"}</div>
            {/each}
            {#if filtered.length > shown.length}
              <button class="btn more" onclick={() => (limit += PAGE)}>
                Показать ещё {fmtNum(Math.min(PAGE, filtered.length - shown.length))}
              </button>
            {/if}
          </div>
        </div>

        {#if scan.selected}
          <FileDetails file={scan.selected} onclose={() => (scan.selected = null)} />
        {/if}
      </section>
    </div>
  {:else if !scan.running && !scan.error}
    <div class="features appear">
      <div class="card feature">
        <div class="glyph accent"><Icon name="certificate" size={20} /></div>
        <span class="body-strong">Подписи</span>
        <span class="caption">Authenticode через WinVerifyTrust, включая каталоги Windows</span>
      </div>
      <div class="card feature">
        <div class="glyph ok"><Icon name="database" size={20} /></div>
        <span class="body-strong">Эталоны</span>
        <span class="caption">
          {baselines.list.length ? `Сохранено: ${baselines.list.length}. ` : ""}Снимок хешей для сверки
        </span>
        <button class="btn subtle link" onclick={() => (ui.view = "baselines")}>Открыть<Icon name="chevronRight" size={14} /></button>
      </div>
      <div class="card feature">
        <div class="glyph warn"><Icon name="warning" size={20} /></div>
        <span class="body-strong">Эвристики</span>
        <span class="caption">Подмена системных имён, чужой издатель, двойные расширения</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .page {
    max-width: 1080px;
    margin: 0 auto;
    padding: 28px 40px 40px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .title {
    margin-bottom: 4px;
  }

  /* Target card */
  .target {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .row-head {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .head-text {
    display: flex;
    flex-direction: column;
  }

  .target-row {
    display: flex;
    gap: 8px;
  }

  .target-row .btn.accent {
    min-width: 132px;
  }

  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .pill {
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

  .progress {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .pct {
    margin-left: auto;
  }

  .bar {
    position: relative;
    height: 3px;
    margin-top: -6px;
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

  .expander {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 -16px -16px;
    padding: 12px 16px;
    border-top: 1px solid var(--divider);
    border-radius: 0 0 var(--r-card) var(--r-card);
    text-align: left;
  }

  .expander:hover {
    background: var(--subtle-hover);
  }

  .chev {
    margin-left: auto;
    display: flex;
    transition: transform 0.15s;
  }

  .chev.open {
    transform: rotate(180deg);
  }

  .blocklist {
    margin-top: 14px;
  }

  /* Dashboard */
  .dash {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .kpis {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }

  .charts {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .chart {
    padding: 16px;
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
    height: 34px;
    padding: 0 8px;
    border-radius: var(--r-ctl);
    text-align: left;
  }

  .legend-row:hover,
  .legend-row.on {
    background: var(--subtle-hover);
  }

  .legend-row.zero {
    opacity: 0.55;
  }

  .count {
    text-align: right;
    font-weight: 600;
  }

  .share {
    text-align: right;
    color: var(--text-3);
  }

  .pubs {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .pub {
    display: grid;
    grid-template-columns: minmax(0, 150px) 1fr 56px;
    align-items: center;
    gap: 12px;
  }

  .pub-name {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pub-track {
    height: 8px;
    border-radius: 4px;
    background: var(--track);
    overflow: hidden;
  }

  .pub-fill {
    height: 100%;
    min-width: 4px;
    border-radius: 4px;
    background: var(--accent);
  }

  .pub-count {
    text-align: right;
    font-weight: 600;
  }

  .foot {
    margin-top: auto;
    padding-top: 12px;
    border-top: 1px solid var(--divider);
    color: var(--text-3);
  }

  .empty-chart {
    padding: 20px 0;
    color: var(--text-3);
  }

  /* Files */
  .files {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    height: 560px;
    overflow: hidden;
  }

  .files.split {
    grid-template-columns: minmax(0, 1fr) 330px;
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

  /* Never clip a filter: when space runs out, search wraps to its own line. */
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
    flex: 1 1 180px;
    max-width: 280px;
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

  .list {
    flex: 1;
    overflow: auto;
    padding: 4px;
  }

  .item {
    position: relative;
    width: 100%;
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) minmax(0, 200px) 14px;
    align-items: center;
    gap: 12px;
    min-height: 48px;
    padding: 6px 12px;
    border-radius: var(--r-ctl);
    text-align: left;
  }

  .item:hover {
    background: var(--subtle-hover);
  }

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
  .item-sig {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-sub.mono {
    color: var(--text-3);
    font-size: 11.5px;
  }

  .item-sig {
    text-align: right;
  }

  .item-chev {
    display: flex;
    color: var(--text-3);
  }

  .split .item-sig {
    display: none;
  }

  .split .item {
    grid-template-columns: 20px minmax(0, 1fr) 14px;
  }

  .empty {
    padding: 48px 0;
    text-align: center;
  }

  .more {
    display: flex;
    margin: 8px auto;
  }

  /* Idle */
  .features {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }

  .feature {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 16px;
  }

  .feature .glyph {
    margin-bottom: 8px;
  }

  .feature .link {
    margin: 4px 0 -6px -8px;
    padding: 0 8px;
    height: 28px;
    color: var(--accent);
    gap: 4px;
  }

  @media (max-width: 1100px) {
    .kpis {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
