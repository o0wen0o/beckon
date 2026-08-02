<script lang="ts">
  // The follow-up box. It owns its own text and its own height, because both
  // are things only this element knows: clearing the value does not shrink an
  // element that was grown inline, so the reset has to happen where the node is.
  import { Send } from "../lib/icons";

  let {
    placeholder,
    disabled,
    onsend,
  }: { placeholder: string; disabled: boolean; onsend: (text: string) => void } = $props();

  let draft = $state("");
  let box = $state<HTMLTextAreaElement | null>(null);

  /** Five rows. Mirrored by `max-height` below, which stops the *scroller*
   *  growing past the same point once the inline height is capped. */
  const MAX_H = 120;

  export function focus() {
    requestAnimationFrame(() => box?.focus());
  }

  /** Called when the window is revealed for a new trigger (ADR-0007). */
  export function reset() {
    draft = "";
    if (box) box.style.height = "";
  }

  /** Grow with the text up to five rows, then scroll inside the box. */
  function grow() {
    if (!box) return;
    box.style.height = "auto";
    box.style.height = `${Math.min(box.scrollHeight, MAX_H)}px`;
  }

  function send() {
    const text = draft.trim();
    if (text === "" || disabled) return;
    reset();
    onsend(text);
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      send();
    }
  }
</script>

<footer>
  <div class="composer">
    <textarea
      bind:this={box}
      bind:value={draft}
      oninput={grow}
      {onkeydown}
      rows="1"
      {placeholder}
    ></textarea>
    <button class="primary send" disabled={draft.trim() === "" || disabled} onclick={send}>
      <Send size={14} /> Send
    </button>
  </div>
</footer>

<style>
  footer {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3);
    border-top: 1px solid var(--border);
    background: var(--bg-raised);
    border-radius: 0 0 var(--surface-radius) var(--surface-radius);
    background-clip: padding-box;
  }

  .composer {
    display: flex;
    align-items: flex-end;
    gap: var(--space-2);
  }

  /* One row deep to start, and exactly as tall as the button beside it — both
     read `--control-h`, so the pair cannot drift apart. It grows with the text
     from there, and the button stays put at the bottom of the row. */
  .composer textarea {
    min-height: var(--control-h);
    height: var(--control-h);
    max-height: 120px;
    padding-top: var(--space-2);
    padding-bottom: var(--space-2);
    resize: none;
  }

  .send {
    flex: none;
    height: var(--control-h);
  }
</style>
