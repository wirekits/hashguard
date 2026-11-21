<script lang="ts">
  import type { FileReport } from "../lib/types";
  import { formatBytes, sigLabel, splitPath } from "../lib/format";
  import { revealInExplorer } from "../lib/api";
  import LevelPill, { levelTone } from "./LevelPill.svelte";
  import Icon from "./Icon.svelte";
  import HashBytes from "./HashBytes.svelte";

  let { file, onclose }: { file: FileReport; onclose: () => void } = $props();

  let copied = $state(false);
  const parts = $derived(splitPath(file.path));

  async function copyHash() {
    try {
      await navigator.clipboard.writeText(file.sha256);
      copied = true;
      setTimeout(() => (copied = false), 1400);
    } catch {
      /* clipboard may be unavailable */
    }
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={onkey} />

<aside class="details" aria-label="File details">
  <header>
    <LevelPill level={file.level} />
    <button class="icon-btn" onclick={onclose} aria-label="Close"><Icon name="close" size={14} /></button>
  </header>

  <div class="scroll">
    <h3 class="subtitle selectable" title={parts.name}>{parts.name}</h3>
    <div class="caption mono selectable dir">{parts.dir}</div>

    {#if file.findings.length}
      <ul class="findings {levelTone[file.level]}">
        {#each file.findings as f}<li>{f}</li>{/each}
      </ul>
    {/if}

    <dl>
      <div class="prop">
        <dt class="caption">Signature</dt>
        <dd>
          {sigLabel[file.signature.status]}
          {#if file.signature.via_catalog}<div class="caption">via Windows catalog</div>{/if}
          {#if file.signature.detail}<div class="caption">{file.signature.detail}</div>{/if}
        </dd>
      </div>
      {#if file.signature.signer}
        <div class="prop">
          <dt class="caption">Publisher</dt>
          <dd class="selectable">{file.signature.signer}</dd>
        </div>
      {/if}
      <div class="prop">
        <dt class="caption">Size</dt>
        <dd class="num">{formatBytes(file.size)}</dd>
      </div>
      <div class="prop">
        <dt class="caption">SHA-256</dt>
        <dd class="hash"><HashBytes hash={file.sha256} /></dd>
      </div>
    </dl>
  </div>

  <footer>
    <button class="btn" onclick={copyHash}>
      <Icon name={copied ? "check" : "copy"} size={16} />{copied ? "Copied" : "Copy hash"}
    </button>
    <button class="btn" onclick={() => revealInExplorer(file.path)}>
      <Icon name="folder" size={16} />Show in folder
    </button>
  </footer>
</aside>

<style>
  .details {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-left: 1px solid var(--divider);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 8px 0 20px;
    height: 48px;
  }
  .scroll {
    flex: 1;
    overflow: auto;
    padding: 4px 20px 16px;
  }
  h3 {
    word-break: break-all;
  }
  .dir {
    margin-top: 2px;
    color: var(--text-3);
    word-break: break-all;
  }
  .findings {
    margin: 16px 0 0;
    padding: 10px 14px 10px 30px;
    border-radius: var(--r-card);
    background: var(--neutral-bg);
    font-size: 13px;
    line-height: 18px;
  }
  .findings.warn {
    background: var(--warn-bg);
  }
  .findings.danger {
    background: var(--danger-bg);
  }
  .findings.info {
    background: var(--info-bg);
  }
  .findings li + li {
    margin-top: 6px;
  }
  .findings li::first-letter {
    text-transform: uppercase;
  }
  dl {
    margin: 16px 0 0;
  }
  .prop {
    padding: 10px 0;
    border-top: 1px solid var(--divider);
  }
  dt {
    color: var(--text-3);
  }
  dd {
    margin: 2px 0 0;
    word-break: break-word;
  }
  dd .caption::first-letter {
    text-transform: uppercase;
  }
  .hash {
    margin-top: 8px;
  }
  footer {
    display: flex;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--divider);
  }
  footer .btn {
    flex: 1;
    min-width: 0;
  }
</style>
