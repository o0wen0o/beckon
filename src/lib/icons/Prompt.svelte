<script lang="ts">
  // Input Source: `prompt` — typed input only.
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
  <rect x="3" y="4.5" width="18" height="15" rx="2.5" />
  <path d="m7.5 9.5 3 2.5-3 2.5M13 14.5h4" />
</svg>
