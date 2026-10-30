<script lang="ts">
  import { formatAccelerator, toAccelerator } from "../../shortcut";

  let {
    accelerator,
    invalid = false,
    defaultAccelerator,
    onstart,
    onrecord,
    onreset,
  }: {
    /** The accelerator shown while the recorder is idle. */
    accelerator: string;
    /** Draws the error ring (the error text is the tab's). */
    invalid?: boolean;
    defaultAccelerator: string;
    /** The user began recording. */
    onstart: () => void;
    /** A valid combination was pressed. */
    onrecord: (accelerator: string) => void;
    onreset: () => void;
  } = $props();

  let recording = $state(false);

  function start() {
    recording = true;
    onstart();
  }

  function onkeydown(event: KeyboardEvent) {
    if (!recording) {
      // The reset button inside the recorder keeps its own Enter and Space.
      if (event.target !== event.currentTarget) return;
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        start();
      }
      return;
    }
    // Every key belongs to the recorder while it listens.
    event.preventDefault();
    event.stopPropagation();
    const bare = !(event.metaKey || event.ctrlKey || event.altKey || event.shiftKey);
    if (event.code === "Escape" && bare) {
      recording = false;
      return;
    }
    const recorded = toAccelerator(event);
    if (recorded === null) return; // Still listening: a modifier alone or a key without ⌘, ⌃, ⌥.
    recording = false;
    onrecord(recorded);
  }

  function reset(event: MouseEvent) {
    event.stopPropagation();
    recording = false;
    onreset();
  }
</script>

<div
  class="shortcut-recorder"
  class:is-recording={recording}
  class:has-error={invalid && !recording}
  id="shortcut-recorder"
  role="button"
  tabindex="0"
  aria-labelledby="shortcut-label shortcut-keys"
  aria-describedby="shortcut-help"
  onclick={start}
  {onkeydown}
  onblur={() => (recording = false)}
>
  <span id="shortcut-keys">{recording ? "Type shortcut…" : formatAccelerator(accelerator)}</span>
  <button
    type="button"
    class="shortcut-reset"
    id="btn-shortcut-reset"
    aria-label="Restore default shortcut"
    title="Restore default ({formatAccelerator(defaultAccelerator)})"
    onclick={reset}
    ><svg class="ico" viewBox="0 0 16 16" aria-hidden="true"
      ><circle cx="8" cy="8" r="6.5" fill="currentColor" stroke="none" /><path
        d="m5.8 5.8 4.4 4.4m0-4.4-4.4 4.4"
        stroke="var(--surface-window)"
        stroke-width="1.5"
      /></svg
    ></button
  >
</div>
