<script lang="ts">
  import BrowserHost from "./BrowserHost.svelte";
</script>

<section id="popover" aria-label="Tadidy clipboard history">
  <BrowserHost label="popover" />
</section>

<style>
  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  :global(html, body, #app) {
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
  }
</style>
