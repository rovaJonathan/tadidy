<script lang="ts">
  import BrowserHost from "./BrowserHost.svelte";
</script>

<section id="quick-panel" aria-label="Tadidy Quick Panel">
  <BrowserHost label="panel" />
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

  #quick-panel {
    width: 100%;
    height: 100%;
    border-radius: var(--radius-window);
    overflow: hidden;
  }
</style>
