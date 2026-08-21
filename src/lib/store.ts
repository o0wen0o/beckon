// The reactivity primitive the Settings stores are built on, now that the
// surface is React. It exists so those stores stay plain classes: every rule
// about when a snapshot may be adopted is written once, in the store, and a
// field component cannot acquire an opinion about the disk (ADR-0003).
//
// `useSyncExternalStore` rather than putting the state in components: there is
// exactly one Settings window and it is never destroyed (ADR-0007), so a
// module-level singleton outliving every render is the honest shape — and it is
// what lets `settings:opened` reset the last visit's leftovers from outside the
// React tree.

/** A store components can subscribe to. Mutating methods call `notify`. */
export class Notifier {
  #listeners = new Set<() => void>();
  // The snapshot `useSyncExternalStore` compares. The fields themselves are
  // read straight off the store; only the fact that something moved is
  // published, so no per-field selector has to be kept in step.
  #version = 0;

  readonly subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  readonly getVersion = (): number => this.#version;

  protected notify() {
    this.#version += 1;
    for (const listener of this.#listeners) listener();
  }
}
