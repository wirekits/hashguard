<script lang="ts" module>
  // Outline icons on a 20px grid, drawn to match Fluent System Icons' weight.
  const C = (cx: number, cy: number, r: number) =>
    `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;
  const SHIELD = "M10 2.5 4 4.8v4.6c0 3.9 2.6 7.1 6 8.1 3.4-1 6-4.2 6-8.1V4.8L10 2.5z";

  export const ICONS = {
    search: [C(8.5, 8.5, 5.5), "m12.6 12.6 4.4 4.4"],
    shield: [SHIELD],
    shieldCheck: [SHIELD, "m7.3 10 2 2 3.6-3.8"],
    shieldError: [SHIELD, "M10 6.6v4", "M10 13.3v.1"],
    warning: ["M8.7 3.7a1.5 1.5 0 0 1 2.6 0l6 10.6a1.5 1.5 0 0 1-1.3 2.2H4a1.5 1.5 0 0 1-1.3-2.2z", "M10 7.6v3.8", "M10 13.9v.1"],
    info: [C(10, 10, 7.5), "M10 9.2v4.6", "M10 6.4v.1"],
    folder: ["M2.5 5.5A1.5 1.5 0 0 1 4 4h3.6l1.6 1.8H16a1.5 1.5 0 0 1 1.5 1.5v7.2A1.5 1.5 0 0 1 16 16H4a1.5 1.5 0 0 1-1.5-1.5z"],
    files: ["M6 2.5h5.5L15 6v9.5A1.5 1.5 0 0 1 13.5 17h-7.5A1.5 1.5 0 0 1 4.5 15.5V4A1.5 1.5 0 0 1 6 2.5z", "M11.5 2.5V6H15", "M7.5 10h5", "M7.5 13h3.5"],
    clock: [C(10, 10, 7.5), "M10 6v4l2.6 1.6"],
    check: ["m4.5 10.5 3.5 3.5 7.5-8"],
    chevronRight: ["m8 5 5 5-5 5"],
    chevronDown: ["m5 8 5 5 5-5"],
    database: ["M4 5c0 1.4 2.7 2.5 6 2.5s6-1.1 6-2.5-2.7-2.5-6-2.5S4 3.6 4 5z", "M4 5v10c0 1.4 2.7 2.5 6 2.5s6-1.1 6-2.5V5", "M4 10c0 1.4 2.7 2.5 6 2.5s6-1.1 6-2.5"],
    trash: ["M4 5.5h12", "M8 5.5V4h4v1.5", "m5.5 5.5.7 10.2a1.5 1.5 0 0 0 1.5 1.3h4.6a1.5 1.5 0 0 0 1.5-1.3l.7-10.2"],
    download: ["M10 3v9.5", "m6.5 9 3.5 3.5L13.5 9", "M4 15v.5A1.5 1.5 0 0 0 5.5 17h9a1.5 1.5 0 0 0 1.5-1.5V15"],
    close: ["m5 5 10 10M15 5 5 15"],
    copy: ["M8.5 7h6A1.5 1.5 0 0 1 16 8.5v7a1.5 1.5 0 0 1-1.5 1.5h-6A1.5 1.5 0 0 1 7 15.5v-7A1.5 1.5 0 0 1 8.5 7z", "M13 7V5.5A1.5 1.5 0 0 0 11.5 4h-6A1.5 1.5 0 0 0 4 5.5v6A1.5 1.5 0 0 0 5.5 13H7"],
    stop: ["M7 5.5h6A1.5 1.5 0 0 1 14.5 7v6a1.5 1.5 0 0 1-1.5 1.5H7A1.5 1.5 0 0 1 5.5 13V7A1.5 1.5 0 0 1 7 5.5z"],
    certificate: ["M4.5 3.5h11A1.5 1.5 0 0 1 17 5v7a1.5 1.5 0 0 1-1.5 1.5H12", "M8 13.5H4.5A1.5 1.5 0 0 1 3 12V5a1.5 1.5 0 0 1 1.5-1.5", "M6.5 7h7", C(10, 13, 2), "m8.8 14.8-.8 2.7 2-1 2 1-.8-2.7"],
    grid: ["M4.5 3h3A1.5 1.5 0 0 1 9 4.5v3A1.5 1.5 0 0 1 7.5 9h-3A1.5 1.5 0 0 1 3 7.5v-3A1.5 1.5 0 0 1 4.5 3z", "M12.5 3h3A1.5 1.5 0 0 1 17 4.5v3A1.5 1.5 0 0 1 15.5 9h-3A1.5 1.5 0 0 1 11 7.5v-3A1.5 1.5 0 0 1 12.5 3z", "M4.5 11h3A1.5 1.5 0 0 1 9 12.5v3A1.5 1.5 0 0 1 7.5 17h-3A1.5 1.5 0 0 1 3 15.5v-3A1.5 1.5 0 0 1 4.5 11z", "M12.5 11h3a1.5 1.5 0 0 1 1.5 1.5v3a1.5 1.5 0 0 1-1.5 1.5h-3a1.5 1.5 0 0 1-1.5-1.5v-3a1.5 1.5 0 0 1 1.5-1.5z"],
    list: ["M7 5h10", "M7 10h10", "M7 15h10", "M3.5 5v.1", "M3.5 10v.1", "M3.5 15v.1"],
  } satisfies Record<string, string[]>;

  export type IconName = keyof typeof ICONS;
</script>

<script lang="ts">
  let { name, size = 16 }: { name: IconName; size?: number } = $props();
</script>

<svg
  viewBox="0 0 20 20"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each ICONS[name] as d}<path {d} />{/each}
</svg>
