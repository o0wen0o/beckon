import { mount } from "svelte";
import "../app.css";
import Popover from "./Popover.svelte";

export default mount(Popover, { target: document.getElementById("app")! });
