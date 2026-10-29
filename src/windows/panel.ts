import { mount } from "svelte";
import "../styles/tokens.css";
import Panel from "./Panel.svelte";
import { followSystemTheme } from "./theme";

const target = document.getElementById("app");
if (!target) throw new Error("#app is missing from panel.html");

followSystemTheme();

export default mount(Panel, { target });
