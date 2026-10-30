<script lang="ts">
  import type { SettingsStore } from "../../stores/settings.svelte";

  let { store }: { store: SettingsStore } = $props();
  void store;
</script>

<div></div>
