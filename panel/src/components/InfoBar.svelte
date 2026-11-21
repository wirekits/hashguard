<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon, { type IconName } from "./Icon.svelte";

  type Severity = "info" | "ok" | "warn" | "danger";
  let {
    severity = "info",
    title,
    children,
  }: { severity?: Severity; title: string; children?: Snippet } = $props();

  const icons: Record<Severity, IconName> = {
    info: "info",
    ok: "shieldCheck",
    warn: "warning",
    danger: "shieldError",
  };
</script>

<div class="infobar {severity}" role={severity === "danger" ? "alert" : "status"}>
  <span class="ico c-{severity}"><Icon name={icons[severity]} size={16} /></span>
  <div class="text">
    <span class="body-strong">{title}</span>
    {#if children}<span class="msg">{@render children()}</span>{/if}
  </div>
</div>

<style>
  .infobar {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 13px 16px;
    border-radius: var(--r-card);
    border: 1px solid var(--stroke);
    background: var(--neutral-bg);
  }
  .infobar.info {
    background: var(--info-bg);
  }
  .infobar.ok {
    background: var(--ok-bg);
  }
  .infobar.warn {
    background: var(--warn-bg);
  }
  .infobar.danger {
    background: var(--danger-bg);
  }
  .ico {
    display: flex;
    padding-top: 2px;
  }
  .text {
    display: flex;
    flex-wrap: wrap;
    column-gap: 12px;
    row-gap: 2px;
  }
  .msg {
    color: var(--text-2);
  }
</style>
