<script lang="ts">
  // Dev harness: later UI tickets mount their components here for end-to-end tests.
  import { appVersion } from "./lib/api";

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
</main>
