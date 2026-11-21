<script lang="ts">
  import Icon, { type IconName } from "../components/Icon.svelte";
  import LevelPill from "../components/LevelPill.svelte";
  import InfoBar from "../components/InfoBar.svelte";
  import type { Level } from "../lib/types";

  const checks: { icon: IconName; tone: string; title: string; text: string }[] = [
    {
      icon: "certificate",
      tone: "accent",
      title: "Authenticode signatures",
      text: "Verified with WinVerifyTrust. When a file has no embedded signature, hashguard looks it up in the signed Windows catalogs (.cat), exactly like Windows does.",
    },
    {
      icon: "database",
      tone: "ok",
      title: "Baselines",
      text: "The SHA-256 of every executable in a folder is saved on a clean system and later compared with the current state.",
    },
    {
      icon: "warning",
      tone: "warn",
      title: "Heuristics",
      text: "A system process name outside system folders, a foreign publisher on a 'system' file, a signature that no longer matches, double extensions like invoice.pdf.exe.",
    },
  ];

  const levels: { level: Level; text: string }[] = [
    { level: "Ok", text: "Valid signature, nothing unusual" },
    { level: "Info", text: "Worth knowing, not a reason for alarm" },
    { level: "Warn", text: "Unsigned where a signature is expected, or the certificate chain does not build" },
    { level: "Danger", text: "Modified after signing, masquerading as a system file, or on the blocklist" },
  ];

  const stack = ["Rust", "Tauri 2", "Svelte 5", "windows-sys", "rayon"];
</script>

<div class="page">
  <h1 class="title">About</h1>

  <div class="layout">
    <section class="card hero">
      <svg viewBox="0 0 24 24" width="72" height="72" aria-hidden="true">
        <path fill="var(--accent)" d="M12 2.5 4.5 5.3v5.6c0 4.7 3.2 9 7.5 10.6 4.3-1.6 7.5-5.9 7.5-10.6V5.3L12 2.5z" />
        <path fill="none" stroke="var(--on-accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" d="m8.6 12.2 2.4 2.4 4.4-4.6" />
      </svg>
      <div class="subtitle">hashguard</div>
      <div class="caption">Version 0.3.1 · MIT license</div>
      <p>Integrity checking for Windows executables: signatures, hashes, baselines and heuristics.</p>
      <div class="chips">
        {#each stack as s}<span class="chip caption">{s}</span>{/each}
      </div>
    </section>

    <div class="col">
      <h2 class="section">What gets checked</h2>
      <div class="stack">
        {#each checks as c}
          <div class="card setting">
            <div class="glyph {c.tone}"><Icon name={c.icon} size={20} /></div>
            <div class="setting-text">
              <span class="body-strong">{c.title}</span>
              <span class="caption">{c.text}</span>
            </div>
          </div>
        {/each}
      </div>

      <h2 class="section">Severity levels</h2>
      <div class="card levels">
        {#each levels as l}
          <div class="level-row">
            <LevelPill level={l.level} />
            <span class="caption">{l.text}</span>
          </div>
        {/each}
      </div>

      <div class="note">
        <InfoBar severity="info" title="This is not an antivirus.">
          A file with no findings is not proven safe: malicious code can carry a genuine signature.
          A finding means "take a look at this file", not "this is a virus".
        </InfoBar>
      </div>
    </div>
  </div>
</div>

<style>
  .page {
    padding: 28px 36px 36px;
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(280px, 1fr) 3fr;
    gap: 16px;
    align-items: start;
    margin-top: 20px;
  }
  .hero {
    position: sticky;
    top: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 32px 24px;
    text-align: center;
  }
  .hero svg {
    margin-bottom: 12px;
  }
  .hero p {
    margin: 10px 0 0;
    color: var(--text-2);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 6px;
    margin-top: 18px;
  }
  .chip {
    padding: 2px 10px;
    border-radius: 12px;
    background: var(--neutral-bg);
    border: 1px solid var(--stroke);
  }
  .col {
    min-width: 0;
  }
  .col .section:first-child {
    margin-top: 0;
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
    padding: 14px 16px;
    box-shadow: none;
  }
  .setting-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .levels {
    padding: 4px 16px;
    box-shadow: none;
  }
  .level-row {
    display: grid;
    grid-template-columns: 130px 1fr;
    align-items: center;
    gap: 16px;
    padding: 12px 0;
  }
  .level-row + .level-row {
    border-top: 1px solid var(--divider);
  }
  .note {
    margin-top: 20px;
  }
  @media (max-width: 1100px) {
    .layout {
      grid-template-columns: 1fr;
    }
    .hero {
      position: static;
    }
  }
</style>
