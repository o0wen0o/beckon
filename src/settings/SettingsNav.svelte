<script lang="ts">
  // Four sections and nothing else: the Actions used to sit here as a second
  // list, which made "pick a section" and "pick an Action" the same gesture for
  // two things that are not alike. Authoring an Action now happens in the
  // Launcher, beside the list it is picked from.
  import { BrandMark, Folder, Keyboard, Palette, Plug, Sliders } from "../lib/icons";
  import { revealConfigDir } from "../lib/ipc";
  import { settings, type SectionRoute } from "./store.svelte";

  const SECTIONS: { id: SectionRoute; label: string; icon: typeof Plug }[] = [
    { id: "connection", label: "Connection", icon: Plug },
    { id: "triggering", label: "Triggering", icon: Keyboard },
    { id: "appearance", label: "Appearance", icon: Palette },
    { id: "defaults", label: "Model defaults", icon: Sliders },
  ];

  const route = $derived(settings.route);

  /** A section is flagged when something inside it needs attention, so a
   *  problem in a pane you are not looking at is still discoverable. A degraded
   *  model list is amber, not red: the dropdown still works. */
  const sectionFlag = $derived.by(
    (): Record<SectionRoute, "bad" | "warn" | null> => ({
      connection:
        settings.keyStatus?.kind === "read-error" || settings.test.state === "failed"
          ? "bad"
          : settings.keyStatus?.kind === "no-credential"
            ? "warn"
            : null,
      triggering: settings.startupErrors.length > 0 ? "bad" : null,
      appearance: null,
      defaults: settings.models !== null && !settings.models.live ? "warn" : null,
    }),
  );
</script>

<nav aria-label="Settings">
  <div class="brand">
    <BrandMark size={20} />
    <span>Beckon</span>
  </div>

  <ul class="sections">
    {#each SECTIONS as section (section.id)}
      {@const Icon = section.icon}
      {@const active = route === section.id}
      <li>
        <button
          class="nav-item"
          class:active
          aria-current={active ? "page" : undefined}
          onclick={() => settings.go(section.id)}
        >
          <span class="rail"></span>
          <Icon size={15} />
          <span class="nav-label">{section.label}</span>
          {#if sectionFlag[section.id]}
            <span
              class="flag"
              data-tone={sectionFlag[section.id]}
              title="Something in this section needs attention"
            ></span>
          {/if}
        </button>
      </li>
    {/each}
  </ul>

  <div class="foot">
    <button class="quiet" onclick={() => revealConfigDir()}>
      <Folder size={14} /> Open folder
    </button>
  </div>
</nav>

<style>
  nav {
    width: 240px;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--bg-raised);
    padding: var(--space-3) var(--space-2);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2) var(--space-4);
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: var(--weight-semibold);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-item {
    position: relative;
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    justify-content: flex-start;
    text-align: left;
    border: none;
    background: none;
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    color: var(--text-dim);
  }

  .nav-item:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: transparent;
    color: var(--text);
  }

  .nav-item.active {
    background: var(--bg-hover);
    color: var(--text);
    font-weight: var(--weight-medium);
  }

  /* Decorative: the tint is what says "current". */
  .rail {
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 2px;
    border-radius: var(--radius-pill);
    background: linear-gradient(var(--brand-from), var(--brand-to));
    opacity: 0;
    transition: opacity var(--dur-fast) var(--ease-out);
  }

  .nav-item.active .rail {
    opacity: 1;
  }

  .nav-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .flag {
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 50%;
  }

  .flag[data-tone="bad"] {
    background: var(--danger);
  }

  .flag[data-tone="warn"] {
    background: var(--warn);
  }

  .foot {
    margin-top: auto;
    padding-top: var(--space-2);
    border-top: 1px solid var(--border);
  }

  .foot button {
    width: 100%;
    justify-content: flex-start;
  }
</style>
