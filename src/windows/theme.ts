// The theme follows the system until Preferences › Appearance can override it.
export function followSystemTheme(): void {
  const system = window.matchMedia("(prefers-color-scheme: dark)");
  const applyTheme = () => {
    document.documentElement.dataset.theme = system.matches ? "dark" : "light";
  };
  applyTheme();
  system.addEventListener("change", applyTheme);
}
