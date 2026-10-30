<script lang="ts">
  // The clip browser wired to the core's commands. The Popover and the Quick Panel both put it in their window (QPN-06).
  import { onMount } from "svelte";
  import { copyClip, hideWindow, onPanelShown, openPreferences, pasteClip } from "../lib/api";
  import ClipBrowser from "../lib/components/ClipBrowser.svelte";
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

  /** A failed command shows nothing in the MVP; in development its code reaches the console, never its message. */
  function report(command: string, error: unknown) {
    if (!import.meta.env.DEV) return;
    const code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : "unknown";
    console.error(`${command} failed: ${code}`);
  }

  /** On `pasted` Rust has already hidden the window; on `copied_only` it stays open and the banner shows. */
  function paste(id: string) {
    pasteClip(id, "default").catch((error) => report("paste_clip", error));
  }

  function copy(id: string) {
    copyClip(id).catch((error) => report("copy_clip", error));
  }

  function close() {
    hideWindow().catch((error) => report("hide_window", error));
  }

  function preferences() {
    openPreferences().catch((error) => report("open_preferences", error));
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
