# Beckon

Beckon is a tool that lives in the background on Windows: press a hotkey to summon it, send a preset prompt plus your current input to DeepSeek, and get the result streamed back in a popover next to your cursor. The name comes from "beckoning" — you wave, it comes.

## Language

### Core concepts

**Action**:
A preset prompt together with how it is triggered. This is the basic unit users configure and invoke.
_Avoid_: Skill, Command, Preset, feature

**Input Source**:
A property of an Action declaring where its input comes from — `selection` (selection only), `prompt` (typed input only), `auto` (use the selection if there is one, otherwise ask for typed input).
_Avoid_: input mode, mode

**Selection**:
The text the user has highlighted in a program outside Beckon, obtained by simulating Ctrl+C.
_Avoid_: highlighted text, selected region, text grab

### Triggering

**Launcher**:
The searchable list of Actions summoned by the global hotkey. It is the universal entry point to every Action.
_Avoid_: panel, main window, palette

**Direct Hotkey**:
A dedicated hotkey bound to a single Action. Its only reason to exist is zero interaction — once pressed, the user waits for the result and makes no choices.
_Avoid_: hotkey, shortcut (both are ambiguous and may refer to the global hotkey)

### Execution and presentation

**Popover**:
The lightweight floating window that pops up near the cursor. It hosts the input box, the streamed result, and any follow-up questions. Closing it destroys it.
_Avoid_: result window, floating window, toast, panel

**Exchange**:
The multi-turn conversation opened by one Action trigger, with a lifetime equal to that of the Popover. Discarded when the window closes; never persisted.
_Avoid_: session, Session, Conversation, history
