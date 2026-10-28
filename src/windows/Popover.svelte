<script lang="ts">
  // Placeholder until the clip browser lands in TAD-016; for now it proves the window.
  import { hideWindow } from "../lib/api";

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") void hideWindow();
  }
</script>

<svelte:window {onkeydown} />

<section id="popover" aria-label="Tadidy clipboard history">
  <p>Tadidy</p>
</section>

<style>
  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  :global(html, body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
    /* The native vibrancy shows through: the webview stays transparent. */
    background: transparent;
  }

  :global(body) {
    font-family: var(--font-sans);
    font-size: var(--text-body);
    line-height: var(--leading-body);
    color: var(--color-text-primary);
    -webkit-font-smoothing: antialiased;
  }

  #popover {
    width: 100%;
    height: 100%;
    border-radius: var(--radius-window);
    overflow: hidden;
    display: grid;
    place-items: center;
  }
</style>
