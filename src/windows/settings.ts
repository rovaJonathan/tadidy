import { mount } from "svelte";
import "../styles/tokens.css";
import Settings from "./Settings.svelte";
import { followSystemTheme } from "./theme";

const target = document.getElementById("app");
if (!target) throw new Error("#app is missing from settings.html");

followSystemTheme();

export default mount(Settings, { target });
