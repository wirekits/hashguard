<script lang="ts">
  // A SHA-256 drawn as 32 byte cells. With `against`, every byte that differs
  // from the other hash lights up, so a modified file reads at a glance.
  let { hash, against = "" }: { hash: string; against?: string } = $props();

  const bytes = $derived(hash.match(/.{1,2}/g) ?? []);
  const other = $derived(against ? (against.match(/.{1,2}/g) ?? []) : []);
  const changed = $derived(against ? bytes.filter((b, i) => b !== other[i]).length : 0);
</script>

<div class="bytes" role="img" aria-label={against ? `${hash}, ${changed} bytes differ` : hash}>
  {#each bytes as b, i}
    <span class="b" class:diff={!!against && b !== other[i]}>{b}</span>
  {/each}
</div>

<style>
  .bytes {
    display: grid;
    grid-template-columns: repeat(16, minmax(0, 1fr));
    gap: 3px;
  }
  .b {
    display: grid;
    place-items: center;
    height: 22px;
    border-radius: 5px;
    background: var(--ctl);
    border: 1px solid var(--ctl-stroke);
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-2);
  }
  .b.diff {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 700;
    box-shadow: 0 0 14px rgba(255, 255, 255, 0.4);
  }
</style>
