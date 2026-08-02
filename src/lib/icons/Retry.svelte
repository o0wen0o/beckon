<script lang="ts">
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
  <path d="M20 5v6h-6" />
  <path d="M19.4 13a7.5 7.5 0 1 1-1.9-6.6L20 9" />
</svg>
