<script lang="ts">
  // A section-scoped message: something about this pane, not about the window.
  // Window-level state lives in the status bar instead, so the two cannot pile
  // up into a wall of coloured boxes.
  import type { Snippet } from "svelte";
  import { BrandMark, Warning } from "../icons";

  interface Props {
    tone?: "info" | "warn" | "danger";
    children: Snippet;
  }

  let { tone = "info", children }: Props = $props();
</script>

<div class="callout" data-tone={tone} role={tone === "danger" ? "alert" : undefined}>
  <span class="icon">
    {#if tone === "info"}<BrandMark size={16} />{:else}<Warning size={16} />{/if}
  </span>
  <div class="text">{@render children()}</div>
</div>

<style>
  .callout {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    border: 1px solid var(--border);
    border-left-width: 3px;
    border-radius: var(--radius-md);
    background: var(--bg-raised);
    padding: var(--space-3);
    margin-bottom: var(--space-4);
  }

  .callout[data-tone="info"] {
    border-left-color: var(--accent);
  }

  .callout[data-tone="warn"] {
    border-left-color: var(--warn);
  }

  .callout[data-tone="warn"] .icon {
    color: var(--warn);
  }

  .callout[data-tone="danger"] {
    border-left-color: var(--danger);
  }

  .callout[data-tone="danger"] .icon {
    color: var(--danger);
  }

  .icon {
    display: flex;
    padding-top: 2px;
  }

  .text {
    min-width: 0;
  }

  .text :global(p) {
    margin: 0 0 var(--space-1);
  }

  .text :global(p:last-child) {
    margin-bottom: 0;
  }

  .text :global(ul) {
    margin: var(--space-1) 0 0;
    padding-left: var(--space-5);
  }
</style>
