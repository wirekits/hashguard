<script lang="ts">
  import Icon, { type IconName } from "./Icon.svelte";

  let {
    icon,
    tone = "neutral",
    label,
    value,
    sub = "",
  }: { icon: IconName; tone?: string; label: string; value: string; sub?: string } = $props();
</script>

<div class="tile card">
  <div class="glyph {tone}"><Icon name={icon} size={20} /></div>
  <div class="text">
    <span class="caption">{label}</span>
    <span class="value num">{value}</span>
    {#if sub}<span class="caption sub">{sub}</span>{/if}
  </div>
</div>

<style>
  .tile {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 16px;
    min-width: 0;
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .value {
    font-family: var(--font-display);
    font-size: 28px;
    line-height: 36px;
    font-weight: 600;
  }
  .sub {
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
