import { mount } from "svelte";
import "../app.css";
import { startTheme } from "../lib/theme";
import Settings from "./Settings.svelte";

// Awaited before mounting so the surface never paints light and then flips.
await startTheme();

export default mount(Settings, { target: document.getElementById("app")! });
