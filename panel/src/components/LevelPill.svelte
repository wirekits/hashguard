<script lang="ts" module>
  import type { Level } from "../lib/types";
  import type { IconName } from "./Icon.svelte";

  export const levelIcon: Record<Level, IconName> = {
    Ok: "shieldCheck",
    Info: "info",
    Warn: "warning",
    Danger: "shieldError",
  };

  export const levelTone: Record<Level, string> = {
    Ok: "ok",
    Info: "info",
    Warn: "warn",
    Danger: "danger",
  };
</script>

<script lang="ts">
  import { levelLabel } from "../lib/format";
  import Icon from "./Icon.svelte";

  // Status is never color alone: icon + label, colored together.
  let { level, label = true }: { level: Level; label?: boolean } = $props();
</script>

<span class="lvl c-{levelTone[level]}" title={label ? undefined : levelLabel[level]}>
  <Icon name={levelIcon[level]} size={16} />
  {#if label}<span>{levelLabel[level]}</span>{/if}
</span>

<style>
  .lvl {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    font-size: 13px;
    flex-shrink: 0;
  }
</style>
