<script lang="ts">
  // A switch rather than a checkbox: these settings take effect immediately,
  // and a checkbox reads as "will be applied when you save" — which there is no
  // way to do here (ADR-0003).
  interface Props {
    checked: boolean;
    label: string;
    id?: string;
    describedBy?: string;
    onchange: (checked: boolean) => void;
  }

  let { checked, label, id, describedBy, onchange }: Props = $props();
</script>

<button
  type="button"
  role="switch"
  class="toggle"
  aria-checked={checked}
  aria-describedby={describedBy}
  {id}
  onclick={() => onchange(!checked)}
>
  <span class="track" class:on={checked}><span class="thumb"></span></span>
  <span class="text">{label}</span>
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
</style>
