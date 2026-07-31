import { mount } from "svelte";
import "../app.css";
import Launcher from "./Launcher.svelte";

export default mount(Launcher, { target: document.getElementById("app")! });
