// What the Launcher knows: the registry, and nothing it can write. Authoring an
// Action is Settings' job (ADR-0003) — the Launcher is summoned by a hotkey to
// pick something and get out of the way, so it holds no draft, no save slot and
// no rule about snapshots arriving mid-keystroke, because nothing types here.
//
// A module-level singleton, because there is exactly one Launcher window and it
// is never destroyed (ADR-0007).
import { getActions } from "../lib/ipc";
import type { RegistrySnapshot } from "../lib/types";

class ActionStore {
  snapshot = $state<RegistrySnapshot>({ actions: [], errors: [], hotkey_errors: {} });

  adoptActions(next: RegistrySnapshot) {
    this.snapshot = next;
  }

  async refresh() {
    this.snapshot = await getActions();
  }
}

export const actions = new ActionStore();
