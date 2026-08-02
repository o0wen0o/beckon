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
  <path d="M4 7h10M18 7h2M4 17h4M12 17h8" />
  <circle cx="16" cy="7" r="2" />
  <circle cx="10" cy="17" r="2" />
</svg>
