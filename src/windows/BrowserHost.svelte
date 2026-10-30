<script lang="ts">
  // The clip browser wired to the core's commands. The Popover and the Quick Panel both put it in their window (QPN-06).
  import { onMount } from "svelte";
  import { copyClip, hideWindow, onPanelShown, openPreferences, pasteClip } from "../lib/api";
  import ClipBrowser from "../lib/components/ClipBrowser.svelte";
  import { reportFailure } from "../lib/report";
  import { SettingsStore } from "../lib/stores/settings.svelte";

  /** The label of the window that hosts the browser: `panel://shown` names it. */
  let { label }: { label: "popover" | "panel" } = $props();

  const settings = new SettingsStore();
  let browser = $state<ClipBrowser>();

  onMount(() => {
    // Unreadable settings leave the delete confirmation on, which is the safe side.
    void settings.start().catch(() => undefined);
    let removed = false;
    let unlisten: (() => void) | undefined;
    void onPanelShown((shown) => {
      if (shown.window === label) void browser?.reset();
    }).then((stop) => (removed ? stop() : (unlisten = stop)));
    return () => {
      removed = true;
      unlisten?.();
      settings.destroy();
    };
  });

  /** Gives the focus back to the search field, e.g. when a dialog of the window closes. */
  export function focusSearch() {
    browser?.focusSearch();
  }

  /** On `pasted` Rust has already hidden the window; on `copied_only` it stays open and the banner shows. */
  function paste(id: string) {
    pasteClip(id, "default").catch((error) => reportFailure("paste_clip", error));
  }

  function copy(id: string) {
    copyClip(id).catch((error) => reportFailure("copy_clip", error));
  }

  function close() {
    hideWindow().catch((error) => reportFailure("hide_window", error));
  }

  function preferences() {
    openPreferences().catch((error) => reportFailure("open_preferences", error));
  }
</script>

<ClipBrowser
  bind:this={browser}
  layout={label}
  confirmBeforeDelete={settings.settings?.confirmBeforeDelete ?? true}
  onpaste={paste}
  oncopy={copy}
  onclose={close}
  onopenpreferences={preferences}
  ondisableconfirm={() => void settings.patch({ confirmBeforeDelete: false })}
/>
