import { mount } from "svelte";
import "../styles/tokens.css";
import Popover from "./Popover.svelte";
import { followSystemTheme } from "./theme";

const target = document.getElementById("app");
if (!target) throw new Error("#app is missing from popover.html");

followSystemTheme();

export default mount(Popover, { target });
