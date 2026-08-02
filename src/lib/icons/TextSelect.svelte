<script lang="ts">
  // Input Source: `selection` — text highlighted in another program.
  let { size = 16 }: { size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  <path d="M5 4H4.5A1.5 1.5 0 0 0 3 5.5V6M19 4h.5A1.5 1.5 0 0 1 21 5.5V6M3 18v.5A1.5 1.5 0 0 0 4.5 20H5M21 18v.5a1.5 1.5 0 0 1-1.5 1.5H19M3 10v4M21 10v4M9 4h6M9 20h6" />
  <path d="M8 9h8M8 12.5h5" />
</svg>
