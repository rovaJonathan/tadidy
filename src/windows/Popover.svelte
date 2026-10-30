<script lang="ts">
  import { onMount } from "svelte";
  import { clearHistory, listClips, onClearHistoryRequested, onPanelShown } from "../lib/api";
  import ClearHistoryDialog from "../lib/components/ClearHistoryDialog.svelte";
  import { reportFailure } from "../lib/report";
  import BrowserHost from "./BrowserHost.svelte";

  let host = $state<BrowserHost>();
  let dialog = $state<ClearHistoryDialog>();
  /** How many clips the history holds while the Clear history confirmation is open; no dialog while it is `undefined`. */
  let clearing = $state<number>();

  onMount(() => {
    let removed = false;
    const unlisten: (() => void)[] = [];
    const listeners = [
      onClearHistoryRequested(() => void askToClear()),
      // A dialog left open would still be there the next time the popover is shown.
      onPanelShown((shown) => {
        if (shown.window === "popover") clearing = undefined;
      }),
    ];
    for (const listener of listeners) void listener.then((stop) => (removed ? stop() : unlisten.push(stop)));
    return () => {
      removed = true;
      for (const stop of unlisten) stop();
    };
  });

  /** The tray menu's Clear history… (HIS-06): the confirmation says how many clips are about to go. */
  async function askToClear() {
    try {
      clearing = (await listClips({ limit: 1 })).total;
    } catch (error) {
      reportFailure("list_clips", error);
    }
  }

  function closeDialog() {
    clearing = undefined;
    host?.focusSearch();
  }

  function confirm(keepPinned: boolean) {
    closeDialog();
    clearHistory(keepPinned).catch((error) => reportFailure("clear_history", error));
  }

  /** Every key goes to an open dialog, wherever the focus is: the clip browser never sees it. */
  function onkeydown(event: KeyboardEvent) {
    if (clearing === undefined) return;
    event.stopPropagation();
    dialog?.handleKey(event);
  }
</script>

<!-- The popover hides when it loses focus (MNU-04), and the dialog with it: nothing is cleared. -->
<svelte:window onkeydowncapture={onkeydown} onblur={() => (clearing = undefined)} />

<section id="popover" aria-label="Tadidy clipboard history">
  <BrowserHost bind:this={host} label="popover" />
  {#if clearing !== undefined}
    <div id="modal-layer">
      <ClearHistoryDialog bind:this={dialog} count={clearing} onconfirm={confirm} oncancel={closeDialog} />
    </div>
  {/if}
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
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: var(--radius-window);
    overflow: hidden;
  }

  /* The prototype's dim layer over the browser while a dialog is open. */
  #modal-layer {
    position: absolute;
    inset: 0;
    z-index: 300;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.06);
  }
</style>
