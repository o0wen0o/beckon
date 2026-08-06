<script lang="ts">
  // A switch rather than a checkbox: these settings take effect immediately,
  // and a checkbox reads as "will be applied when you save" — which there is no
  // way to do here (ADR-0003).
  interface Props {
    checked: boolean;
    label: string;
    id?: string;
    describedBy?: string;
    /**
     * Show the switch's *state* — On / Off — instead of repeating `label`.
     *
     * Inside a `Field` the label is already above the control, so printing it
     * again beside the switch says the same thing twice ("Autostart" over
     * "Start with Windows") and leaves the switch itself unlabelled in the one
     * way that matters: whether it is on. `label` stays the accessible name.
     */
    showState?: boolean;
    onchange: (checked: boolean) => void;
  }

  let { checked, label, id, describedBy, showState = false, onchange }: Props = $props();
</script>

<button
  type="button"
  role="switch"
  class="toggle"
  aria-checked={checked}
  aria-describedby={describedBy}
  aria-label={showState ? label : undefined}
  {id}
  onclick={() => onchange(!checked)}
>
  <span class="track" class:on={checked}><span class="thumb"></span></span>
  <!-- aria-hidden when it is the state: the switch already announces checked,
       and a screen reader reading "On" after "on" is noise. -->
  <span class="text" class:state={showState} aria-hidden={showState ? "true" : undefined}>
    {showState ? (checked ? "On" : "Off") : label}
  </span>
</button>

<style>
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    border: none;
    background: none;
    padding: 0;
    align-self: flex-start;
  }

  .toggle:hover:not(:disabled) {
    background: none;
    border-color: transparent;
  }

  .track {
    position: relative;
    width: 34px;
    height: 18px;
    flex: none;
    border-radius: var(--radius-pill);
    background: var(--bg-sunken);
    border: 1px solid var(--border-strong);
    transition:
      background var(--dur-fast) var(--ease-out),
      border-color var(--dur-fast) var(--ease-out);
  }

  .track.on {
    background: linear-gradient(135deg, var(--brand-from), var(--brand-to));
    border-color: transparent;
  }

  .thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--bg-raised);
    box-shadow: var(--shadow-sm);
    transition: transform var(--dur-fast) var(--ease-out);
  }

  .track.on .thumb {
    transform: translateX(16px);
  }

  .text {
    font-size: var(--text-base);
    color: var(--text);
  }

  /* Two characters that swap under a moving thumb: fixed width, or the row
     twitches sideways every time the switch is thrown. */
  .text.state {
    min-width: 2.2em;
    text-align: left;
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
  }
</style>
