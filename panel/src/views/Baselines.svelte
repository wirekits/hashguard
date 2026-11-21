<script lang="ts">
  import { baselines, makeBaseline, runVerify, removeBaseline } from "../lib/app.svelte";
  import { pickFolder } from "../lib/api";
  import { fmtNum, formatDate, pluralFiles } from "../lib/format";
  import Icon from "../components/Icon.svelte";
  import InfoBar from "../components/InfoBar.svelte";
  import ProgressRing from "../components/ProgressRing.svelte";

  let confirming = $state("");

  async function choose() {
    const p = await pickFolder("Папка для эталона");
    if (p) baselines.path = p;
  }

  function askDelete(name: string) {
    if (confirming === name) {
      confirming = "";
      removeBaseline(name);
    } else {
      confirming = name;
      setTimeout(() => {
        if (confirming === name) confirming = "";
      }, 3000);
    }
  }

  const diff = $derived(baselines.result?.diff);
  const changes = $derived(diff ? diff.modified.length + diff.added.length + diff.removed.length : 0);
</script>

<div class="page">
  <h1 class="title">Эталоны</h1>
  <p class="lead">
    Снимок SHA-256 всех исполняемых файлов, сделанный на чистой системе. Сверка покажет каждый
    изменённый, новый и удалённый файл.
  </p>

  <section class="card create">
    <div class="row-head">
      <div class="glyph accent"><Icon name="database" size={20} /></div>
      <div class="head-text">
        <span>Новый эталон</span>
        <span class="caption">Лучше снимать сразу после установки системы или обновлений</span>
      </div>
    </div>
    <div class="form">
      <input
        class="textbox name"
        placeholder="Название"
        bind:value={baselines.name}
        onkeydown={(e) => e.key === "Enter" && makeBaseline()}
        spellcheck="false"
        aria-label="Название эталона"
      />
      <input
        class="textbox mono"
        placeholder="C:\Windows\System32"
        bind:value={baselines.path}
        onkeydown={(e) => e.key === "Enter" && makeBaseline()}
        spellcheck="false"
        aria-label="Папка"
      />
      <button class="btn" onclick={choose}>Обзор…</button>
      <button
        class="btn accent"
        onclick={makeBaseline}
        disabled={!!baselines.busy || !baselines.name.trim() || !baselines.path.trim()}
      >
        {#if baselines.busy === "create"}<ProgressRing size={16} />{/if}
        {baselines.busy === "create" ? "Снимаю…" : "Создать"}
      </button>
    </div>
  </section>

  {#if baselines.error}
    <InfoBar severity="danger" title="Ошибка">{baselines.error}</InfoBar>
  {/if}

  <h2 class="section">Сохранённые</h2>
  <div class="stack">
    {#each baselines.list as b (b.name)}
      <div class="card setting" class:active={baselines.result?.baseline.name === b.name}>
        <div class="glyph"><Icon name="shieldCheck" size={20} /></div>
        <div class="setting-text">
          <span>{b.name}</span>
          <span class="caption mono root" title={b.root}>{b.root}</span>
          <span class="caption num">{pluralFiles(b.files)} · {formatDate(b.created_unix)}</span>
        </div>
        <div class="actions">
          <button class="btn" onclick={() => runVerify(b.name)} disabled={!!baselines.busy}>
            {#if baselines.verifying === b.name}<ProgressRing size={16} />Сверяю…{:else}Сверить{/if}
          </button>
          <button
            class="btn subtle"
            class:danger-text={confirming === b.name}
            onclick={() => askDelete(b.name)}
            disabled={!!baselines.busy}
            aria-label="Удалить эталон {b.name}"
          >
            <Icon name="trash" size={16} />{confirming === b.name ? "Удалить?" : ""}
          </button>
        </div>
      </div>
    {:else}
      <div class="card setting empty">
        <div class="glyph"><Icon name="info" size={20} /></div>
        <span class="caption">Эталонов пока нет. Сними первый, пока система чистая.</span>
      </div>
    {/each}
  </div>

  {#if baselines.result && diff}
    <h2 class="section">Результат сверки · {baselines.result.baseline.name}</h2>
    <div class="result appear">
      <InfoBar
        severity={changes ? (diff.modified.length ? "danger" : "warn") : "ok"}
        title={changes ? `Найдено изменений: ${fmtNum(changes)}` : "Все файлы совпадают с эталоном"}
      >
        {#if changes}
          Изменено {fmtNum(diff.modified.length)} · добавлено {fmtNum(diff.added.length)} · удалено {fmtNum(diff.removed.length)}
        {:else}
          {baselines.result.root}
        {/if}
      </InfoBar>

      {#if changes}
        <div class="card changes">
          {#each diff.modified as m (m.path)}
            <div class="change">
              <span class="tag c-danger"><Icon name="shieldError" size={16} />Изменён</span>
              <div class="change-body">
                <span class="mono selectable">{m.path}</span>
                <span class="caption mono selectable old">было {m.old_sha256}</span>
                <span class="caption mono selectable">стало {m.new_sha256}</span>
              </div>
            </div>
          {/each}
          {#each diff.added as a (a)}
            <div class="change">
              <span class="tag c-warn"><Icon name="warning" size={16} />Новый</span>
              <span class="mono selectable change-body">{a}</span>
            </div>
          {/each}
          {#each diff.removed as r (r)}
            <div class="change">
              <span class="tag c-info"><Icon name="info" size={16} />Удалён</span>
              <span class="mono selectable change-body">{r}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page {
    max-width: 1080px;
    margin: 0 auto;
    padding: 28px 40px 40px;
  }
  .lead {
    margin: 6px 0 20px;
    max-width: 640px;
    color: var(--text-2);
  }
  .create {
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
  .form {
    display: flex;
    gap: 8px;
  }
  .form .name {
    flex: 0 0 220px;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .setting {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 68px;
    padding: 12px 16px;
    box-shadow: none;
  }
  .setting.active {
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .setting.empty .glyph {
    width: 32px;
    height: 32px;
  }
  .setting-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .root {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .actions .btn.subtle {
    height: 32px;
    padding: 0 8px;
  }
  .result {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .changes {
    padding: 4px 16px;
  }
  .change {
    display: grid;
    grid-template-columns: 110px minmax(0, 1fr);
    gap: 12px;
    padding: 12px 0;
  }
  .change + .change {
    border-top: 1px solid var(--divider);
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    font-size: 13px;
    align-self: start;
  }
  .change-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    word-break: break-all;
  }
  .old {
    text-decoration: line-through;
    color: var(--text-3);
  }
</style>
