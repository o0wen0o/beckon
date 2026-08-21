// The reactivity primitive the stores are built on. It exists so they stay
// plain classes: every rule about when a snapshot may be adopted lives in the
// store, and no field component acquires an opinion about the disk (ADR-0003).
//
// `useSyncExternalStore` over component state because each window is created
// once and never destroyed (ADR-0007), so a module-level singleton outlives
// every render — which is what lets `settings:opened` reset it from outside the
// React tree.

/** A store components can subscribe to. Mutating methods call `notify`. */
export class Notifier {
  #listeners = new Set<() => void>();
  // The snapshot `useSyncExternalStore` compares. Fields are read straight off
  // the store; only "something moved" is published.
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
