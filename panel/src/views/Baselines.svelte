<script lang="ts">
  import { baselines, makeBaseline, runVerify, removeBaseline } from "../lib/app.svelte";
  import { pickFolder } from "../lib/api";
  import { fmtNum, formatDate, pluralFiles } from "../lib/format";
  import Icon from "../components/Icon.svelte";
  import InfoBar from "../components/InfoBar.svelte";
  import ProgressRing from "../components/ProgressRing.svelte";
  import HashBytes from "../components/HashBytes.svelte";

  let confirming = $state("");

  async function choose() {
    const p = await pickFolder("Folder for the baseline");
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
  <h1 class="title">Baselines</h1>
  <p class="lead">
    A baseline is a SHA-256 snapshot of every executable in a folder, taken while the system is
    clean. Verifying against it shows every file that was modified, added or removed since.
  </p>

  <div class="layout">
    <div class="col">
      <section class="card create">
        <div class="row-head">
          <div class="glyph accent"><Icon name="database" size={20} /></div>
          <div class="head-text">
            <span class="body-strong">New baseline</span>
            <span class="caption">Best taken right after installing Windows or updates</span>
          </div>
        </div>
        <label class="field">
          <span class="caption">Name</span>
          <input
            class="textbox"
            placeholder="system32-clean"
            bind:value={baselines.name}
            onkeydown={(e) => e.key === "Enter" && makeBaseline()}
            spellcheck="false"
          />
        </label>
        <label class="field">
          <span class="caption">Folder</span>
          <span class="inline">
            <input
              class="textbox mono"
              placeholder="C:\Windows\System32"
              bind:value={baselines.path}
              onkeydown={(e) => e.key === "Enter" && makeBaseline()}
              spellcheck="false"
            />
            <button class="btn" onclick={choose}>Browse…</button>
          </span>
        </label>
        <button
          class="btn accent wide"
          onclick={makeBaseline}
          disabled={!!baselines.busy || !baselines.name.trim() || !baselines.path.trim()}
        >
          {#if baselines.busy === "create"}<ProgressRing size={16} />Hashing files…{:else}Create baseline{/if}
        </button>
      </section>

      {#if baselines.error}
        <InfoBar severity="danger" title="Something went wrong">{baselines.error}</InfoBar>
      {/if}

      <h2 class="section">Saved baselines</h2>
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
                {#if baselines.verifying === b.name}<ProgressRing size={16} />Verifying…{:else}Verify{/if}
              </button>
              <button
                class="btn subtle"
                class:danger-text={confirming === b.name}
                onclick={() => askDelete(b.name)}
                disabled={!!baselines.busy}
                aria-label="Delete baseline {b.name}"
              >
                <Icon name="trash" size={16} />{confirming === b.name ? "Delete?" : ""}
              </button>
            </div>
          </div>
        {:else}
          <div class="card setting">
            <div class="glyph"><Icon name="info" size={20} /></div>
            <span class="caption">No baselines yet. Take the first one while the system is clean.</span>
          </div>
        {/each}
      </div>
    </div>

    <section class="card result">
      {#if baselines.result && diff}
        <div class="appear result-inner">
          <div class="result-head">
            <span class="caption">Verification</span>
            <span class="subtitle">{baselines.result.baseline.name}</span>
            <span class="caption mono">{baselines.result.root}</span>
          </div>
          <InfoBar
            severity={changes ? (diff.modified.length ? "danger" : "warn") : "ok"}
            title={changes ? `${fmtNum(changes)} ${changes === 1 ? "change" : "changes"} found` : "Everything matches the baseline"}
          >
            {#if changes}
              {fmtNum(diff.modified.length)} modified · {fmtNum(diff.added.length)} added · {fmtNum(diff.removed.length)} removed
            {:else}
              {pluralFiles(baselines.result.baseline.files)} checked
            {/if}
          </InfoBar>

          {#if changes}
            <div class="changes">
              {#each diff.modified as m (m.path)}
                <div class="change">
                  <span class="tag c-danger"><Icon name="shieldError" size={16} />Modified</span>
                  <div class="change-body">
                    <span class="mono selectable">{m.path}</span>
                    <span class="eyebrow">Expected</span>
                    <HashBytes hash={m.old_sha256} />
                    <span class="eyebrow">Actual</span>
                    <HashBytes hash={m.new_sha256} against={m.old_sha256} />
                  </div>
                </div>
              {/each}
              {#each diff.added as a (a)}
                <div class="change">
                  <span class="tag c-warn"><Icon name="warning" size={16} />Added</span>
                  <span class="mono selectable change-body">{a}</span>
                </div>
              {/each}
              {#each diff.removed as r (r)}
                <div class="change">
                  <span class="tag c-info"><Icon name="info" size={16} />Removed</span>
                  <span class="mono selectable change-body">{r}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {:else}
        <div class="placeholder">
          <div class="glyph"><Icon name={baselines.busy === "verify" ? "clock" : "shieldCheck"} size={20} /></div>
          <span class="body-strong">{baselines.busy === "verify" ? "Re-hashing files…" : "No verification yet"}</span>
          <span class="caption">Press Verify on a saved baseline to compare it with the folder as it is now.</span>
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .page {
    padding: 28px 36px 36px;
  }
  .lead {
    margin: 6px 0 20px;
    max-width: 760px;
    color: var(--text-2);
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(380px, 5fr) 7fr;
    gap: 16px;
    align-items: start;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }
  .create {
    padding: 18px;
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
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .inline {
    display: flex;
    gap: 8px;
  }
  .wide {
    align-self: flex-start;
    min-width: 160px;
  }
  .section {
    margin: 12px 0 0;
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
    position: sticky;
    top: 0;
    min-height: 480px;
    padding: 20px;
  }
  .result-inner {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .result-head {
    display: flex;
    flex-direction: column;
  }
  .placeholder {
    height: 100%;
    min-height: 440px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    text-align: center;
  }
  .placeholder .caption {
    max-width: 320px;
  }
  .changes {
    display: flex;
    flex-direction: column;
  }
  .change {
    display: grid;
    grid-template-columns: 110px minmax(0, 1fr);
    gap: 12px;
    padding: 12px 0;
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
  .change-body .eyebrow {
    margin-top: 8px;
  }
  @media (max-width: 1100px) {
    .layout {
      grid-template-columns: 1fr;
    }
    .result {
      position: static;
    }
  }
</style>
