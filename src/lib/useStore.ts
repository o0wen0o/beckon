// Subscribe a component to a `Notifier` store. Returns the store, so the call
// reads as "this component renders from that store".
import { useSyncExternalStore } from "react";
import type { Notifier } from "./store";

export function useStore<S extends Notifier>(store: S): S {
  useSyncExternalStore(store.subscribe, store.getVersion, store.getVersion);
  return store;
}
