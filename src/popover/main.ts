import { mount } from "svelte";
import "../app.css";
import { startTheme } from "../lib/theme";
import Popover from "./Popover.svelte";

// Awaited before mounting so the surface never paints light and then flips.
// This window is created hidden at startup (ADR-0007), so the wait is paid once
// at launch and never on the hot path.
await startTheme();

export default mount(Popover, { target: document.getElementById("app")! });
