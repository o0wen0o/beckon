// Transient reports that are about an *action the user just took*, rather than
// about a field.
//
// The distinction is the whole reason this exists beside `Callout` and
// `StatusBar` rather than replacing either. A Callout is about the pane and
// stays as long as the condition does; the status bar is about the window and
// the write protocol (ADR-0003). Neither fits "the endpoint answered" — an
// outcome with no field to sit under, which was being rendered as a sentence
// growing sideways out of the button that caused it and pushing the row it
// belonged to off its own line.
//
// Nothing here is a control: a toast carries a sentence and dismisses itself.
// Anything a user must act on is a field's error, which stays on the field.
import { Notifier } from "@/lib/store";

/** `Callout`'s vocabulary plus the outcome it has no reason to name: a Callout
 *  is only ever raised by a problem, and a toast reports success too. */
export type ToastTone = "ok" | "warn" | "danger";

export interface Toast {
  id: number;
  tone: ToastTone;
  message: string;
}

/**
 * How long each tone stands.
 *
 * A success is read at a glance and its absence says the same thing; a failure
 * is a sentence naming a cause, and quoting a cause verbatim is the point of
 * `describeFailure` — so it stays long enough to read twice. Both are
 * dismissable, and neither may be the *only* place a lasting condition is
 * stated: a missing key is still on the key field after this has gone.
 */
const LIFETIME: Record<ToastTone, number> = {
  ok: 4000,
  warn: 9000,
  danger: 9000,
};

class ToastStore extends Notifier {
  items: readonly Toast[] = [];

  #next = 1;
  #timers = new Map<number, number>();

  show(tone: ToastTone, message: string) {
    const id = this.#next++;
    this.items = [...this.items, { id, tone, message }];
    this.#timers.set(
      id,
      window.setTimeout(() => this.dismiss(id), LIFETIME[tone]),
    );
    this.notify();
  }

  dismiss(id: number) {
    const timer = this.#timers.get(id);
    if (timer !== undefined) {
      window.clearTimeout(timer);
      this.#timers.delete(id);
    }
    if (!this.items.some((one) => one.id === id)) return;
    this.items = this.items.filter((one) => one.id !== id);
    this.notify();
  }

  /** Every window is reused rather than destroyed (ADR-0007), so a reopened
   *  Settings would otherwise resume the last visit's outcomes as if they had
   *  just happened. Called from `resetTransient` with the rest of them. */
  clear() {
    for (const timer of this.#timers.values()) window.clearTimeout(timer);
    this.#timers.clear();
    if (this.items.length === 0) return;
    this.items = [];
    this.notify();
  }
}

export const toasts = new ToastStore();
