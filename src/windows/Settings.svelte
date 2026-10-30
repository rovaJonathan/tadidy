<script lang="ts">
  import { onMount } from "svelte";
  import "../styles/preferences.css";
  import GeneralTab from "../lib/components/preferences/GeneralTab.svelte";
  import ShortcutsTab from "../lib/components/preferences/ShortcutsTab.svelte";
  import { SettingsStore } from "../lib/stores/settings.svelte";

  // Only the tabs of the MVP: Capture, Appearance, Storage & Privacy and About come with phase 2.
  const TABS = [
    { id: "general", label: "General", color: "#8E8E93" },
    { id: "shortcuts", label: "Shortcuts", color: "#5E5CE6" },
  ] as const;
  type TabId = (typeof TABS)[number]["id"];

  const store = new SettingsStore();
  let selected = $state<TabId>("general");
  const title = $derived(TABS.find((tab) => tab.id === selected)?.label);

  onMount(() => {
    // Unreadable settings leave the panels empty; the window itself still opens and closes.
    void store.start().catch(() => undefined);
    return () => store.destroy();
  });

  function select(id: TabId) {
    selected = id;
    document.getElementById(`prefs-tab-${id}`)?.focus();
  }

  /** ↑ and ↓ move through the tabs and wrap around (DESIGN.md §5). */
  function onkeydown(event: KeyboardEvent) {
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const index = TABS.findIndex((tab) => tab.id === selected);
    const next = TABS[(index + step + TABS.length) % TABS.length];
    if (next) select(next.id);
  }
</script>

<section id="preferences" aria-label="Tadidy Preferences">
  <nav class="prefs-sidebar" data-tauri-drag-region>
    <div
      class="prefs-nav"
      role="tablist"
      aria-orientation="vertical"
      aria-label="Preferences sections"
      {onkeydown}
      tabindex="-1"
    >
      {#each TABS as tab (tab.id)}
        <button
          type="button"
          class="prefs-nav-item"
          role="tab"
          id="prefs-tab-{tab.id}"
          aria-controls="prefs-{tab.id}"
          aria-selected={selected === tab.id}
          tabindex={selected === tab.id ? 0 : -1}
          data-tab={tab.id}
          onclick={() => select(tab.id)}
        >
          <span class="sidebar-icon" style:background={tab.color}>
            {#if tab.id === "general"}
              <svg class="ico" viewBox="0 0 16 16" aria-hidden="true"
                ><circle
                  cx="8"
                  cy="8"
                  r="5.3"
                  stroke-width="2.6"
                  stroke-dasharray="2.08 2.08"
                  stroke-linecap="butt"
                /><circle cx="8" cy="8" r="2" /></svg
              >
            {:else}
              <svg class="ico" viewBox="0 0 16 16" aria-hidden="true"
                ><rect x="1.5" y="4" width="13" height="8.5" rx="1.5" /><path
                  d="M4.5 7h1M7.5 7h1M10.5 7h1M5.5 10h5"
                /></svg
              >
            {/if}
          </span>
          {tab.label}
        </button>
      {/each}
    </div>
  </nav>

  <div class="prefs-main">
    <header class="prefs-header" data-tauri-drag-region><h1 id="prefs-title">{title}</h1></header>
    <div class="prefs-content">
      <GeneralTab {store} hidden={selected !== "general"} />
      <ShortcutsTab {store} hidden={selected !== "shortcuts"} />
    </div>
  </div>
</section>

<style>
  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  :global(html, body, #app) {
    margin: 0;
    height: 100%;
    overflow: hidden;
  }

  :global(body) {
    font-family: var(--font-sans);
    font-size: var(--text-body);
    line-height: var(--leading-body);
    color: var(--color-text-primary);
    -webkit-font-smoothing: antialiased;
    background: var(--surface-window);
  }

  :global(button, select, input) {
    font-family: inherit;
  }

  #preferences {
    height: 100%;
    display: grid;
    grid-template-columns: var(--prefs-sidebar-width) minmax(0, 1fr);
    background: var(--surface-window);
  }
</style>
