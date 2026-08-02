<script lang="ts">
  // A hotkey recorder that registers what it records **immediately**: if the
  // combination is taken, it goes red on the spot and the value is refused
  // (README). Nothing unregisterable can reach disk through this component.
  import { Keyboard } from "../icons";
  import { describeError, probeHotkey } from "../ipc";

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
    class:recording
    class:invalid={error !== null}
    onclick={() => (recording ? stop() : start())}
    onkeydown={onKeydown}
    onblur={stop}
  >
    <Keyboard size={14} />
    {#if recording}
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

  .recorder > button:first-child {
    min-width: 190px;
    justify-content: flex-start;
    font-variant-numeric: tabular-nums;
  }

  .recording {
    border-color: var(--accent);
    color: var(--accent);
  }

  .invalid {
    border-color: var(--danger);
    color: var(--danger);
  }

  .link {
    border: none;
    background: none;
    color: var(--text-dim);
    text-decoration: underline;
    padding: 2px var(--space-1);
  }

  .link:hover:not(:disabled) {
    background: none;
    border-color: transparent;
    color: var(--accent);
  }
</style>
