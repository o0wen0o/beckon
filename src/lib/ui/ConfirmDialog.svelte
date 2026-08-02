<script lang="ts">
  // Built on the native `<dialog>` element: the focus trap, Esc-to-dismiss, the
  // backdrop and top-layer stacking all come from the platform, so this needs
  // no dependency and no hand-rolled focus management.
  //
  // It replaces `confirm()`, which WebView2 renders as unthemed browser chrome
  // with the app origin in the title, blocks the whole webview including any
  // in-flight debounced save, and cannot name the file in the app's own voice.
  import type { Snippet } from "svelte";

  interface Props {
    open: boolean;
    title: string;
    body: Snippet;
    confirmLabel: string;
    destructive?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  }

  let {
    open,
    title,
    body,
    confirmLabel,
    destructive = false,
    onconfirm,
    oncancel,
  }: Props = $props();

  let dialog = $state<HTMLDialogElement | null>(null);

  $effect(() => {
    if (!dialog) return;
    // Guarded on the element's own state, not on the previous prop value:
    // showModal() throws if it is already open, and close() on a closed dialog
    // is a no-op.
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  });
</script>

<dialog
  bind:this={dialog}
  oncancel={(event) => {
    event.preventDefault();
    oncancel();
  }}
  onclose={() => oncancel()}
>
  <h2>{title}</h2>
  <div class="body">{@render body()}</div>
  <div class="buttons">
    <!-- Cancel takes focus: the default action of a destructive dialog must
         not be the destructive one. The usual objection to autofocus — that it
         moves focus without the user asking — does not apply inside a modal
         the user just opened, and `showModal()` has to put focus somewhere. -->
    <!-- svelte-ignore a11y_autofocus -->
    <button autofocus onclick={oncancel}>Cancel</button>
    <button class:danger={destructive} onclick={onconfirm}>{confirmLabel}</button>
  </div>
</dialog>

<style>
  dialog {
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--bg-raised);
    color: var(--text);
    padding: var(--space-5);
    max-width: 420px;
    box-shadow: var(--shadow-lg);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.45);
  }

  h2 {
    margin: 0 0 var(--space-2);
    font-family: var(--font-display);
    font-size: var(--text-md);
  }

  .body {
    color: var(--text-dim);
    font-size: var(--text-sm);
  }

  .body :global(p) {
    margin: 0 0 var(--space-1);
  }

  .body :global(code) {
    font-size: var(--text-sm);
    color: var(--text);
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }

  .buttons .danger {
    border-color: var(--danger);
    color: var(--danger);
  }
</style>
