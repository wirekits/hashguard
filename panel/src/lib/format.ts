import type { Level, SigStatus } from "./types";

export const levelLabel: Record<Level, string> = {
  Ok: "Чисто",
  Info: "Инфо",
  Warn: "Внимание",
  Danger: "Опасно",
};

export const sigLabel: Record<SigStatus, string> = {
  Valid: "Подписан",
  Unsigned: "Без подписи",
  Invalid: "Подпись недействительна",
  Error: "Не удалось проверить",
};

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} Б`;
  const units = ["КБ", "МБ", "ГБ"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms} мс`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)} с`;
  return `${Math.floor(s / 60)} мин ${Math.round(s % 60)} с`;
}

export function formatDate(unix: number): string {
  return new Date(unix * 1000).toLocaleString("ru-RU", {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

/** Split `C:\a\b\file.dll` into directory and file name. */
export function splitPath(p: string): { dir: string; name: string } {
  const i = Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/"));
  return i < 0 ? { dir: "", name: p } : { dir: p.slice(0, i), name: p.slice(i + 1) };
}

export const fmtNum = (n: number) => n.toLocaleString("ru-RU");

// The core (shared with the English CLI) reports findings in English.
// Translate the known messages for the panel; unknown text passes through.
const detailRules: [RegExp, (m: RegExpMatchArray) => string][] = [
  [/^no signature present$/, () => "подписи нет"],
  [/^file digest does not match the signature/, () => "хеш не совпадает с подписью: файл изменён после подписания"],
  [/^signing certificate expired$/, () => "срок действия сертификата истёк"],
  [/^certificate chain ends in an untrusted root$/, () => "цепочка ведёт к недоверенному корневому сертификату"],
  [/^certificate chain could not be built/, () => "не удалось построить цепочку сертификатов, имя издателя не подтверждено"],
  [/^explicitly distrusted publisher$/, () => "издатель помечен как недоверенный"],
  [/^certificate revoked$/, () => "сертификат отозван"],
  [/^subject is not trusted for the requested action$/, () => "файл не доверен для этого действия"],
  [/^trust verification failed \((0x[0-9A-F]+)\)$/, (m) => `проверка доверия не пройдена (${m[1]})`],
  [/^signature verification is only supported on Windows$/, () => "проверка подписи доступна только в Windows"],
];

export function tDetail(text: string | null): string {
  if (!text) return "";
  for (const [re, f] of detailRules) {
    const m = text.match(re);
    if (m) return f(m);
  }
  return text;
}

const findingRules: [RegExp, (m: RegExpMatchArray) => string][] = [
  [/^hash is on the blocklist$/, () => "Хеш есть в списке вредоносных"],
  [/^signature check failed: (.*)$/, (m) => `Подпись не прошла проверку: ${tDetail(m[1])}`],
  [/^unsigned binary inside a Windows system directory$/, () => "Нет подписи, хотя файл лежит в системной папке Windows"],
  [/^unsigned binary in a user-writable location$/, () => "Нет подписи, лежит в папке с правом записи для пользователя"],
  [/^unsigned$/, () => "Без подписи"],
  [/^signature could not be checked: (.*)$/, (m) => `Подпись не удалось проверить: ${tDetail(m[1])}`],
  [/^signed by a non-Microsoft publisher \((.*)\) in a system directory$/, (m) => `В системной папке, но издатель не Microsoft: ${m[1]}`],
  [/^'(.+)' is a core Windows process name but lives outside the system directories$/, (m) => `«${m[1]}» — имя системного процесса, но файл лежит вне системных папок`],
  [/^'(.+)' in a system directory is not signed by Microsoft$/, (m) => `«${m[1]}» в системной папке подписан не Microsoft`],
  [/^double extension imitating a document or media file$/, () => "Двойное расширение: маскировка под документ или медиафайл"],
];

export function tFinding(text: string): string {
  for (const [re, f] of findingRules) {
    const m = text.match(re);
    if (m) return f(m);
  }
  return text;
}

export function pluralFiles(n: number): string {
  const m10 = n % 10;
  const m100 = n % 100;
  const word =
    m10 === 1 && m100 !== 11
      ? "файл"
      : m10 >= 2 && m10 <= 4 && (m100 < 10 || m100 >= 20)
        ? "файла"
        : "файлов";
  return `${fmtNum(n)} ${word}`;
}
