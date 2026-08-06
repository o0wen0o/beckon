<script lang="ts">
  // There is no Save button and there never will be (ADR-0003), so this line is
  // where that promise is kept visible — and the one place a failed write is
  // reported, rather than a banner competing with the form above it.
  import { Check, Warning } from "../icons";

  interface Props {
    busy: boolean;
    error: string | null;
    /**
     * What this pane actually promises, when it is not the usual promise. The
     * raw file editor has a Save button of its own (a file that does not parse
     * cannot be written on every keystroke), and the standing line would sit
     * directly beneath it saying the opposite.
     */
    note?: string | null;
  }

  let { busy, error, note = null }: Props = $props();
</script>

<div class="status" class:bad={error !== null} role="status" aria-live="polite">
  {#if error}
    <span class="icon"><Warning size={13} /></span>
    <span>Not saved — {error}</span>
  {:else if busy}
    <span class="spinner"></span>
    <span>Saving…</span>
  {:else}
    <span class="icon"><Check size={13} /></span>
    <span>{note ?? "Changes are written to disk as you make them."}</span>
  {/if}
</div>

<style>
  .status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: 28px;
    flex: none;
    padding: 0 var(--space-4);
    border-top: 1px solid var(--border);
    background: var(--bg-raised);
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
  }

  .status.bad {
    color: var(--danger);
  }

  .icon {
    display: flex;
  }

  .spinner {
    width: 11px;
    height: 11px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    animation: spin 700ms linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation: none;
      border-top-color: var(--border-strong);
    }
  }
</style>
