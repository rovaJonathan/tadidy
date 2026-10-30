<script lang="ts">
  import type { SettingsStore } from "../../stores/settings.svelte";

  let { store, hidden = false }: { store: SettingsStore; hidden?: boolean } = $props();

  /** The prototype's presets; `0` is "Unlimited" (HIS-02). */
  const HISTORY_SIZES = [
    [10, "10 clips"],
    [50, "50 clips"],
    [100, "100 clips"],
    [250, "250 clips"],
    [500, "500 clips"],
    [1000, "1,000 clips"],
    [5000, "5,000 clips"],
    [0, "Unlimited"],
  ] as const;

  const settings = $derived(store.settings);
  const loginItemError = $derived(store.error?.code === "login_item_failed" ? store.error : null);

  async function setLaunchAtLogin(event: Event & { currentTarget: HTMLInputElement }) {
    const box = event.currentTarget;
    const saved = await store.patch({ launchAtLogin: box.checked });
    // A refused change leaves the setting as it was, and Svelte sees nothing to redraw: the box goes back by hand.
    if (!saved) box.checked = store.settings?.launchAtLogin ?? false;
  }

  function setHistorySize(event: Event & { currentTarget: HTMLSelectElement }) {
    void store.patch({ historySize: Number(event.currentTarget.value) });
  }

  async function setConfirmBeforeDelete(event: Event & { currentTarget: HTMLInputElement }) {
    const box = event.currentTarget;
    const saved = await store.patch({ confirmBeforeDelete: box.checked });
    if (!saved) box.checked = store.settings?.confirmBeforeDelete ?? true;
  }
</script>

<div id="prefs-general" class="prefs-panel" role="tabpanel" aria-labelledby="prefs-tab-general" {hidden}>
  {#if settings}
    <div class="pref-group">
      <div class="pref-row">
        <div>
          <label class="pref-label" for="pref-launch-at-login">Launch at login</label>
          {#if loginItemError}
            <p id="pref-launch-at-login-error" class="field-error" role="alert">{loginItemError.message}</p>
          {/if}
        </div>
        <span class="toggle"
          ><input
            type="checkbox"
            role="switch"
            id="pref-launch-at-login"
            checked={settings.launchAtLogin}
            onchange={setLaunchAtLogin}
          /><span class="toggle-track"></span></span
        >
      </div>
      <div class="pref-row">
        <div>
          <label class="pref-label" for="pref-history-size">History size</label>
          <div class="pref-desc">Pinned clips and snippets don’t count toward the limit.</div>
        </div>
        <span class="select-wrap"
          ><select id="pref-history-size" value={String(settings.historySize)} onchange={setHistorySize}>
            {#each HISTORY_SIZES as [size, label] (size)}
              <option value={String(size)}>{label}</option>
            {/each}
          </select></span
        >
      </div>
    </div>
    <div class="pref-group">
      <div class="pref-row">
        <label class="pref-label" for="pref-confirm-delete">Confirm before delete</label>
        <span class="toggle"
          ><input
            type="checkbox"
            role="switch"
            id="pref-confirm-delete"
            checked={settings.confirmBeforeDelete}
            onchange={setConfirmBeforeDelete}
          /><span class="toggle-track"></span></span
        >
      </div>
    </div>
  {/if}
</div>
