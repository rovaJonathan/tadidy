import { mount } from "svelte";
import "../styles/tokens.css";
import Popover from "./Popover.svelte";

const target = document.getElementById("app");
if (!target) throw new Error("#app is missing from popover.html");

// The theme follows the system until Preferences › Appearance can override it.
const system = window.matchMedia("(prefers-color-scheme: dark)");
const applyTheme = () => {
  document.documentElement.dataset.theme = system.matches ? "dark" : "light";
};
applyTheme();
system.addEventListener("change", applyTheme);

export default mount(Popover, { target });
