<script lang="ts">
  // A 0–2 range with meaning at both ends deserves better than a bare spinner.
  // The number input stays: it is the typable, screen-reader-friendly path, and
  // the slider is the affordance.
  interface Props {
    value: number;
    id?: string;
    describedBy?: string;
    onchange: (value: number) => void;
  }

  let { value, id, describedBy, onchange }: Props = $props();

  /**
   * `min`/`max` on a number input constrain the spinner, not typing: "9" is
   * accepted and would reach disk. Clamp on the way out, and drop anything
   * non-finite rather than writing it — the API refuses such a request.
   */
  function commit(raw: string | number) {
    const parsed = typeof raw === "number" ? raw : Number(String(raw).trim());
    if (!Number.isFinite(parsed)) return;
    onchange(Math.min(2, Math.max(0, Math.round(parsed * 10) / 10)));
  }
</script>

<div class="temperature">
  <div class="row">
    <input
      class="slider"
      type="range"
      min="0"
      max="2"
      step="0.1"
      {value}
      aria-label="Temperature"
      aria-describedby={describedBy}
      oninput={(event) => commit(event.currentTarget.value)}
    />
    <input
      class="number"
      type="number"
      step="0.1"
      min="0"
      max="2"
      {id}
      {value}
      oninput={(event) => commit(event.currentTarget.value)}
    />
  </div>
  <div class="scale" aria-hidden="true">
    <span>0 · precise</span>
    <span>1</span>
    <span>2 · loose</span>
  </div>
</div>

<style>
  .temperature {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .slider {
    flex: 1;
    padding: 0;
    border: none;
    background: none;
    accent-color: var(--accent);
  }

  .number {
    width: 76px;
    flex: none;
    font-variant-numeric: tabular-nums;
  }

  .scale {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-small);
    font-size: var(--text-xs);
    color: var(--text-faint);
  }
</style>
