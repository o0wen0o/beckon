// What the Action *is*: its name, what it is for, and what it says to the model
// (ADR-0012). The main screen is the other half — how it fires and what it fires
// at — and these four are the only configurations on the pane that are typed
// rather than chosen. A typed field cannot right-align against its own name, so
// every field here is `stacked`.
//
// One card holds all four, because they are one thing: four boxes inside a screen
// already reached through a box would enclose it four times over. It is also the
// one card on the pane with no hover — there is nothing else here to move to.
//
// And no `measure`: every field runs the card's full width. A measure exists so
// that controls chosen from a set park at one x down a pane of mixed cards; here
// there is one card, nothing to line up against, and the two prompts are the
// longest strings in the app.
//
// No Save button here either (ADR-0003): these commit as you type, like the rest
// of the pane. The store is the same one the main screen writes through.
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { CARD, Field } from "@/components/Field";
import { useStore } from "@/lib/useStore";
import { actionStore } from "../../actions";

/** The screen's name, in one place: the card that opens it and the heading it
 *  opens with have to read as the same thing. What the Action *is* — its name,
 *  what it is for, and what it says to the model — as against the main screen,
 *  which is how it fires and what it fires at. */
export const DEFINITION_SCREEN = "Definition";

export function ActionDefinition() {
  const store = useStore(actionStore);
  const draft = store.draft;

  if (!draft) return null;

  const templateWarning =
    draft.prompt.user && !draft.prompt.user.includes("{{input}}")
      ? "This template never includes the input."
      : null;
  const nameWarning =
    draft.name.trim() === ""
      ? "Without a name this Action shows as its file name in the Launcher."
      : null;

  return (
    // The card's own padding, and the fields spaced by the air a card would have
    // put between them — no inner rule and no inner edge.
    <div className={`${CARD} flex flex-col gap-5`}>
      <Field label="Name" stacked bare warning={nameWarning}>
        {({ id, describedBy }) => (
          <Input
            id={id}
            aria-describedby={describedBy}
            value={draft.name}
            onChange={(event) => {
              const value = event.currentTarget.value;
              store.editDraft((next) => (next.name = value));
            }}
          />
        )}
      </Field>

      <Field
        label="Description"
        stacked
        bare
        hint="Shown under the name in the Launcher, and searched."
      >
        {({ id, describedBy }) => (
          <Input
            id={id}
            aria-describedby={describedBy}
            value={draft.description ?? ""}
            onChange={(event) => {
              const value = event.currentTarget.value || null;
              store.editDraft((next) => (next.description = value));
            }}
          />
        )}
      </Field>

      <Field
        label="System prompt"
        stacked
        bare
        hint="How the model should behave. Sent ahead of every input."
      >
        {({ id, describedBy }) => (
          <Textarea
            id={id}
            aria-describedby={describedBy}
            className="font-mono min-h-30 text-quiet"
            value={draft.prompt.system}
            onChange={(event) => {
              const value = event.currentTarget.value;
              store.editDraft((next) => (next.prompt.system = value));
            }}
          />
        )}
      </Field>

      <Field
        label="User template"
        stacked
        bare
        warning={templateWarning}
        hint="{{input}} is replaced by the Selection or the typed input. Empty means just the input."
      >
        {({ id, describedBy }) => (
          <Input
            id={id}
            aria-describedby={describedBy}
            className="font-mono text-quiet"
            value={draft.prompt.user ?? ""}
            placeholder="{{input}}"
            onChange={(event) => {
              const value = event.currentTarget.value || null;
              store.editDraft((next) => (next.prompt.user = value));
            }}
          />
        )}
      </Field>
    </div>
  );
}
