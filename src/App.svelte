<script lang="ts">
  // Dev harness: later UI tickets mount their components here for end-to-end tests.
  import { appVersion } from "./lib/api";
  import ClipBrowser from "./lib/components/ClipBrowser.svelte";

  let version = $state<string>();

  $effect(() => {
    appVersion().then((v) => (version = v));
  });
</script>

<main>
  <h1>Tadidy</h1>
  {#if version}
    <p>Version {version}</p>
  {/if}
  <!-- The popover's size (DESIGN.md §3), on the window surface the real window gets from vibrancy. -->
  <div class="popover-frame"><ClipBrowser /></div>
</main>

<style>
  :global(body) {
    font-family: var(--font-sans);
    font-size: var(--text-body);
    line-height: var(--leading-body);
    color: var(--color-text-primary);
    -webkit-font-smoothing: antialiased;
  }

  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  .popover-frame {
    width: var(--popover-width);
    height: var(--popover-height);
    border-radius: var(--radius-window);
    overflow: hidden;
    background: var(--surface-window);
    box-shadow: var(--shadow-window);
  }
</style>
