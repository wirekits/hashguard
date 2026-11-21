<script lang="ts">
  import Icon, { type IconName } from "../components/Icon.svelte";
  import LevelPill from "../components/LevelPill.svelte";
  import InfoBar from "../components/InfoBar.svelte";
  import type { Level } from "../lib/types";

  const checks: { icon: IconName; tone: string; title: string; text: string }[] = [
    {
      icon: "certificate",
      tone: "accent",
      title: "Подпись Authenticode",
      text: "Проверка через WinVerifyTrust. Если подписи в файле нет, ищется запись в каталогах Windows (.cat), как это делает сама система.",
    },
    {
      icon: "database",
      tone: "ok",
      title: "Эталоны",
      text: "SHA-256 всех исполняемых файлов папки сохраняется на чистой системе и потом сверяется с текущим состоянием.",
    },
    {
      icon: "warning",
      tone: "warn",
      title: "Эвристики",
      text: "Системное имя вне системных папок, чужой издатель у «системного» файла, подпись не совпадает с содержимым, двойное расширение вида invoice.pdf.exe.",
    },
  ];

  const levels: { level: Level; text: string }[] = [
    { level: "Ok", text: "Подпись действительна, ничего необычного" },
    { level: "Info", text: "Стоит знать, но не повод для тревоги" },
    { level: "Warn", text: "Нет подписи там, где она ожидается, или цепочка сертификатов не строится" },
    { level: "Danger", text: "Файл изменён после подписания, выдаёт себя за системный или есть в списке плохих хешей" },
  ];
</script>

<div class="page">
  <h1 class="title">О программе</h1>

  <section class="card hero">
    <svg viewBox="0 0 24 24" width="56" height="56" aria-hidden="true">
      <path fill="var(--accent)" d="M12 2.5 4.5 5.3v5.6c0 4.7 3.2 9 7.5 10.6 4.3-1.6 7.5-5.9 7.5-10.6V5.3L12 2.5z" />
      <path fill="none" stroke="var(--on-accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" d="m8.6 12.2 2.4 2.4 4.4-4.6" />
    </svg>
    <div>
      <div class="subtitle">hashguard</div>
      <div class="caption">Версия 0.2.0 · Rust, Tauri, Svelte · MIT</div>
      <p>Проверка целостности исполняемых файлов Windows: подписи, хеши, эталоны и эвристики.</p>
    </div>
  </section>

  <h2 class="section">Что проверяется</h2>
  <div class="stack">
    {#each checks as c}
      <div class="card setting">
        <div class="glyph {c.tone}"><Icon name={c.icon} size={20} /></div>
        <div class="setting-text">
          <span>{c.title}</span>
          <span class="caption">{c.text}</span>
        </div>
      </div>
    {/each}
  </div>

  <h2 class="section">Уровни</h2>
  <div class="card levels">
    {#each levels as l}
      <div class="level-row">
        <LevelPill level={l.level} />
        <span class="caption">{l.text}</span>
      </div>
    {/each}
  </div>

  <div class="note">
    <InfoBar severity="info" title="Это не антивирус.">
      Файл без находок не обязательно безопасен: вредоносный код может быть подписан настоящим
      сертификатом. Находка означает «проверь этот файл», а не «это вирус».
    </InfoBar>
  </div>
</div>

<style>
  .page {
    max-width: 1080px;
    margin: 0 auto;
    padding: 28px 40px 40px;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 20px;
    margin-top: 20px;
    padding: 20px;
  }
  .hero p {
    margin: 6px 0 0;
    color: var(--text-2);
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
</style>
