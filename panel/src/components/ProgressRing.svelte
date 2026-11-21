<script lang="ts">
  let { size = 20 }: { size?: number } = $props();
</script>

<svg class="ring" viewBox="0 0 20 20" width={size} height={size} role="progressbar" aria-label="Working">
  <circle cx="10" cy="10" r="8" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" />
</svg>

<style>
  .ring {
    animation: spin 1.6s linear infinite;
    flex-shrink: 0;
  }
  circle {
    stroke-dasharray: 50.3;
    animation: dash 1.6s ease-in-out infinite;
    transform-origin: center;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes dash {
    0% {
      stroke-dashoffset: 48;
    }
    50% {
      stroke-dashoffset: 12;
      transform: rotate(120deg);
    }
    100% {
      stroke-dashoffset: 48;
      transform: rotate(360deg);
    }
  }
</style>
