<script lang="ts">
  // Input Source: `auto` — the Selection if there is one, otherwise typed input.
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
  <path d="M11 3 9.6 7.6 5 9l4.6 1.4L11 15l1.4-4.6L17 9l-4.6-1.4Z" />
  <path d="M17.5 14.5 17 16.5l-2 .5 2 .5.5 2 .5-2 2-.5-2-.5Z" />
  <path d="M5.5 16v3M4 17.5h3" />
</svg>
