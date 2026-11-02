<script lang="ts">
  import { onMount } from "svelte";
  import AppIcon from "./AppIcon.svelte";

  interface Props {
    /** Reset history was chosen. */
    onconfirm: () => void;
    oncancel: () => void;
  }

  const DESCRIPTION =
    "The current history can’t be recovered. Tadidy will start a new, empty one. The old encrypted file is kept in the app’s data folder as tadidy.db.bak.";

  let { onconfirm, oncancel }: Props = $props();

  let dialog = $state<HTMLElement>();
  let cancelButton = $state<HTMLButtonElement>();

  // The safe button has the focus: a stray Return resets nothing.
  onMount(() => cancelButton?.focus());

  function onkeydown(event: KeyboardEvent) {
    // Whatever is pressed in here stays here: the clip browser and the window never see it.
    event.stopPropagation();
    handleKey(event);
  }

  /** The dialog's keys; the clip browser also sends it the keys pressed while the focus is outside it. */
  export function handleKey(event: KeyboardEvent) {
    switch (event.key) {
      case "Escape":
        event.preventDefault();
        oncancel();
        break;
      case "Enter":
        // A focused button does its own click; anywhere else Return is the default button, Cancel.
        if (!(event.target instanceof HTMLButtonElement)) {
          event.preventDefault();
          oncancel();
        }
        break;
      case "Tab":
        event.preventDefault();
        moveFocus(event.shiftKey ? -1 : 1);
        break;
    }
  }

  function moveFocus(step: 1 | -1) {
    const controls = [...(dialog?.querySelectorAll<HTMLElement>("button") ?? [])];
    const at = controls.indexOf(document.activeElement as HTMLElement);
    // Out of the dialog, Tab enters at its first control and ⇧Tab at its last.
    const next = at < 0 ? (step > 0 ? 0 : controls.length - 1) : (at + step + controls.length) % controls.length;
    controls[next]?.focus();
  }
</script>

<div
  id="dialog-reset-history"
  class="alert"
  role="alertdialog"
  aria-modal="true"
  aria-labelledby="dialog-reset-title"
  aria-describedby="dialog-reset-desc"
  tabindex="-1"
  bind:this={dialog}
  {onkeydown}
>
  <div class="alert-icon"><AppIcon /></div>
  <h2 id="dialog-reset-title">Reset clipboard history?</h2>
  <p id="dialog-reset-desc">{DESCRIPTION}</p>
  <div class="alert-buttons">
    <button type="button" class="btn btn-destructive-text" id="btn-reset-confirm" onclick={onconfirm}
      >Reset history</button
    >
    <button type="button" class="btn btn-primary" id="btn-reset-cancel" bind:this={cancelButton} onclick={oncancel}
      >Cancel</button
    >
  </div>
</div>

<style>
  .alert {
    width: var(--alert-width);
    padding: var(--space-20) var(--space-16) var(--space-16);
    border-radius: var(--radius-alert);
    background: var(--material-alert);
    -webkit-backdrop-filter: var(--blur-material);
    backdrop-filter: var(--blur-material);
    box-shadow: var(--shadow-alert);
    text-align: center;
    color: var(--color-text-primary);
    outline: none;
  }

  .alert-icon {
    width: var(--alert-icon-size);
    height: var(--alert-icon-size);
    margin: 0 auto var(--space-10);
  }

  h2 {
    margin: 0 0 var(--space-6);
    font-size: var(--text-body);
    line-height: var(--leading-body);
    font-weight: var(--weight-bold);
  }

  p {
    margin: 0 0 var(--space-12);
    font-size: var(--text-small);
    line-height: var(--leading-small);
  }

  .btn:focus-visible {
    outline: 3px solid var(--color-focus-ring);
  }

  .alert-buttons {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }

  .btn {
    width: 100%;
    height: var(--button-height);
    padding: 0 var(--space-12);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--color-control-bg);
    box-shadow: var(--shadow-control);
    font: inherit;
    font-size: var(--text-body);
    color: var(--color-text-primary);
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .btn:active {
    background: var(--color-fill-pressed);
  }

  .btn-destructive-text {
    color: var(--color-destructive-text);
  }

  .btn-primary {
    background: var(--color-accent);
    color: var(--color-on-accent);
    box-shadow: var(--shadow-primary-button);
  }

  .btn-primary:active {
    background: var(--color-accent);
    filter: brightness(0.92);
  }
</style>
