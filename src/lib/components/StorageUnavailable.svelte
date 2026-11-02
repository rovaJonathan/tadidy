<script lang="ts">
  import EmptyState from "./EmptyState.svelte";

  interface Props {
    /** Reset history… was clicked: the host opens the confirmation. */
    onreset: () => void;
    /** The reset may be offered: the key is lost. Not for a failure that may be temporary. The core decides; no default. */
    canReset: boolean;
    /** The last reset did not work. */
    failed?: boolean;
    /** A reset is running: the button is off. */
    resetting?: boolean;
  }

  let { onreset, canReset, failed = false, resetting = false }: Props = $props();
</script>

<!-- What replaces the list when the history database cannot be opened; the look is the empty states'. -->
<EmptyState variant="storage" {canReset} {failed} {resetting} {onreset} />
