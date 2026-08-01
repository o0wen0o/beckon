<script lang="ts">
  // A hotkey recorder that registers what it records **immediately**: if the
  // combination is taken, it goes red on the spot and the value is refused
  // (README). Nothing unregisterable can reach disk through this component.
  import { describeError, probeHotkey } from "../lib/ipc";

  interface Props {
    value: string | null;
    /** Whether an empty value is allowed (Direct Hotkeys are optional). */
    clearable?: boolean;
    onchange: (accelerator: string | null) => void;
  }

  let { value, clearable = false, onchange }: Props = $props();

  let recording = $state(false);
  let error = $state<string | null>(null);

  function start() {
    recording = true;
    error = null;
  }

  function stop() {
    recording = false;
  }

  async function onKeydown(event: KeyboardEvent) {
    if (!recording) return;
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      stop();
      return;
    }

    const accelerator = toAccelerator(event);
    if (!accelerator) return; // modifiers only so far — keep listening

    try {
      await probeHotkey(accelerator);
      error = null;
      recording = false;
      onchange(accelerator);
    } catch (failure) {
      // Stay in recording mode: the user's next attempt should just work.
      error = describeError(failure).message;
    }
  }

  function clear() {
    error = null;
    onchange(null);
  }

  function toAccelerator(event: KeyboardEvent): string | null {
    const mods: string[] = [];
    if (event.ctrlKey) mods.push("Ctrl");
    if (event.altKey) mods.push("Alt");
    if (event.shiftKey) mods.push("Shift");
    if (event.metaKey) mods.push("Super");

    const key = keyName(event.code);
    if (!key) return null;
    if (mods.length === 0) {
      error = "Add Ctrl, Alt or Shift — a bare key would fire everywhere.";
      return null;
    }
    return [...mods, key].join("+");
  }

  function keyName(code: string): string | null {
    if (/^Key[A-Z]$/.test(code)) return code.slice(3);
    if (/^Digit\d$/.test(code)) return code.slice(5);
    if (/^F\d{1,2}$/.test(code)) return code;
    if (/^(Control|Shift|Alt|Meta|OS)(Left|Right)$/.test(code)) return null;
    const known = [
      "Space",
      "Enter",
      "Tab",
      "Backspace",
      "Delete",
      "Insert",
      "Home",
      "End",
      "PageUp",
      "PageDown",
      "ArrowUp",
      "ArrowDown",
      "ArrowLeft",
      "ArrowRight",
      "Comma",
      "Period",
      "Slash",
      "Semicolon",
      "Quote",
      "Backquote",
      "Backslash",
      "BracketLeft",
      "BracketRight",
      "Minus",
      "Equal",
    ];
    return known.includes(code) ? code : null;
  }
</script>

<div class="recorder">
  <button
    class="slot"
    class:recording
    class:invalid={error !== null}
    onclick={() => (recording ? stop() : start())}
    onkeydown={onKeydown}
    onblur={stop}
  >
    {#if recording}
      <span class="dot"></span>
      Press a combination…
    {:else}
      {value ?? "Not set"}
    {/if}
  </button>

  {#if clearable && value && !recording}
    <button class="link" onclick={clear}>clear</button>
  {/if}
</div>

{#if error}
  <p class="hint error">{error}</p>
{/if}

<style>
  .recorder {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  /* Reads as a field, not a button: it holds a value, and pressing it puts the
     value in play. Its width is fixed so recording does not resize the form. */
  .slot {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 190px;
    min-height: 34px;
    font-weight: 400;
    font-variant-numeric: tabular-nums;
    color: var(--text);
    background: var(--bg-input);
    border-color: transparent;
  }

  .slot:hover:not(:disabled) {
    background: var(--bg-input);
    border-color: var(--border-strong);
  }

  .recording,
  .recording:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
  }

  /* A live recorder is the one place in the app that is waiting on the user
     rather than the other way round, so it gets the same pulse the Popover
     uses for "waiting". */
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.25;
    }
    50% {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .dot {
      opacity: 1;
    }
  }

  .invalid,
  .invalid:hover:not(:disabled) {
    border-color: var(--danger);
    color: var(--danger);
  }

  .link {
    border: none;
    background: none;
    color: var(--text-dim);
    text-decoration: underline;
    padding: 2px 4px;
  }

  .link:hover:not(:disabled) {
    background: none;
    border-color: transparent;
    color: var(--text);
  }
</style>
