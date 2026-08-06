<script lang="ts">
  // A hotkey recorder that registers what it records **immediately**: if the
  // combination is taken, it goes red on the spot and the value is refused
  // (README). Nothing unregisterable can reach disk through this component.
  import { Close, Keyboard } from "../icons";
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
  <!-- The button carries what it does, not just what it holds: styled like a
       field, it reads as a value someone else typed and nothing suggests that
       clicking is how a combination is recorded. -->
  <button
    class="record"
    class:recording
    class:invalid={error !== null}
    onclick={() => (recording ? stop() : start())}
    onkeydown={onKeydown}
    onblur={stop}
  >
    <Keyboard size={14} />
    {#if recording}
      <span class="value">Press a combination…</span>
    {:else}
      <span class="value" class:unset={!value}>{value ?? "Not set"}</span>
      <span class="verb">{value ? "Change" : "Record"}</span>
    {/if}
  </button>

  {#if clearable && value && !recording}
    <button class="quiet clear" aria-label="Clear the Direct Hotkey" onclick={clear}>
      <Close size={13} /> Clear
    </button>
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

  .record {
    min-width: 230px;
    max-width: var(--input-max);
    justify-content: flex-start;
    font-variant-numeric: tabular-nums;
  }

  .value {
    flex: 1;
    min-width: 0;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .value.unset {
    color: var(--text-faint);
  }

  /* The affordance, quiet enough not to compete with the combination itself. */
  .verb {
    flex: none;
    font-family: var(--font-small);
    font-size: var(--text-xs);
    color: var(--text-faint);
  }

  .record:hover:not(:disabled) .verb {
    color: var(--accent);
  }

  .recording {
    border-color: var(--accent);
    color: var(--accent);
  }

  .invalid {
    border-color: var(--danger);
    color: var(--danger);
  }

  .clear {
    flex: none;
    font-family: var(--font-small);
    font-size: var(--text-sm);
  }
</style>
