<script lang="ts">
  import { onMount } from "svelte";
  import { getShortcutStatus } from "../../api";
  import { DEFAULT_QUICK_PANEL_SHORTCUT, isReserved } from "../../shortcut";
  import type { SettingsStore } from "../../stores/settings.svelte";
  import ShortcutRecorder from "./ShortcutRecorder.svelte";

  const CONFLICT = "This shortcut is already used by macOS or another app. Choose another one.";

  let { store, hidden = false }: { store: SettingsStore; hidden?: boolean } = $props();

  /** The combination the user tried and the shortcut could not take; it stays on screen with the error ring. */
  let rejected = $state<string | null>(null);
  /** What the error line says: the conflict, or the core's reason when the refusal is another one. */
  let message = $state(CONFLICT);
  /** The stored shortcut failed to register when Tadidy started. */
  let startupFailed = $state(false);

  const stored = $derived(store.settings?.quickPanelShortcut ?? DEFAULT_QUICK_PANEL_SHORTCUT);
  const failed = $derived(rejected !== null || startupFailed);

  onMount(() => {
    // Without a status there is nothing to flag: the recorder still works and reports a refusal itself.
    void getShortcutStatus()
      .then((status) => {
        startupFailed = status.error !== null;
      })
      .catch(() => undefined);
  });

  function clearError() {
    rejected = null;
    startupFailed = false;
    message = CONFLICT;
  }

  async function save(accelerator: string) {
    // macOS does not refuse a combination it keeps for itself when it is registered: the front knows the list.
    if (isReserved(accelerator)) {
      message = CONFLICT;
      rejected = accelerator;
      return;
    }
    if (!(await store.patch({ quickPanelShortcut: accelerator }))) {
      const { error } = store;
      message = error && error.code !== "shortcut_unavailable" ? error.message : CONFLICT;
      rejected = accelerator;
      return;
    }
    clearError();
  }

  async function reset() {
    // The default already stored has nothing to change, so a startup failure is still there to show.
    rejected = null;
    if (stored !== DEFAULT_QUICK_PANEL_SHORTCUT) await save(DEFAULT_QUICK_PANEL_SHORTCUT);
  }
</script>

<div id="prefs-shortcuts" class="prefs-panel" role="tabpanel" aria-labelledby="prefs-tab-shortcuts" {hidden}>
  <div class="pref-group">
    <div class="pref-row">
      <div>
        <span class="pref-label" id="shortcut-label">Open Quick Panel</span>
        <div class="pref-desc">Works in every app, even in full screen.</div>
        <p id="shortcut-error" class="field-error" role="alert" hidden={!failed}>
          {message}
        </p>
      </div>
      <ShortcutRecorder
        accelerator={rejected ?? stored}
        invalid={failed}
        defaultAccelerator={DEFAULT_QUICK_PANEL_SHORTCUT}
        onstart={clearError}
        onrecord={save}
        onreset={reset}
      />
      <span id="shortcut-help" class="sr-only">Click, then press the new key combination. Press Escape to cancel.</span>
    </div>
  </div>
</div>
