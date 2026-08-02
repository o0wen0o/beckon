<script lang="ts" module>
  // Shared across instances, so each mark gets its own gradient id. Two copies
  // of one id in a document make the second instance reference the first one's
  // defs — the bug that bites the moment the mark appears in a header and an
  // empty state at the same time.
  let counter = 0;
</script>

<script lang="ts">
  // The logo's summoning spark, redrawn as a UI-scale mark: the ring and the
  // four-point core from assets/logo.svg, without the glow filter and drop
  // shadow that collapse into a smear below 32px.
  let { size = 20 }: { size?: number } = $props();
  const id = `beckon-mark-${(counter += 1)}`;
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  aria-hidden="true"
  focusable="false"
>
  <defs>
    <linearGradient {id} x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="var(--brand-from)" />
      <stop offset="100%" stop-color="var(--brand-to)" />
    </linearGradient>
  </defs>
  <path
    d="M12 3.2A8.8 8.8 0 1 1 3.2 12"
    stroke="url(#{id})"
    stroke-width="2"
    stroke-linecap="round"
    opacity="0.55"
  />
  <path d="M12 6.2q0 5.8 5.8 5.8-5.8 0-5.8 5.8Q12 12 6.2 12 12 12 12 6.2Z" fill="url(#{id})" />
</svg>
