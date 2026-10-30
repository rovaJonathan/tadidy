<script lang="ts">
  import { onMount } from "svelte";
  import { clipCountLabel } from "../format";
  import AppIcon from "./AppIcon.svelte";

  interface Props {
    /** How many clips the history holds: the dialog says how many are about to go. */
    count: number;
    /** Clear history was chosen; `keepPinned` is the state of the checkbox. */
    onconfirm: (keepPinned: boolean) => void;
    oncancel: () => void;
  }

  /** After the count, in the prototype's words. */
  const CONSEQUENCE = " will be permanently deleted from this Mac. This can’t be undone.";

  let { count, onconfirm, oncancel }: Props = $props();

  let dialog = $state<HTMLElement>();
  let cancelButton = $state<HTMLButtonElement>();
  let keepPinned = $state(true);

  // The safe button has the focus: a stray Return clears nothing.
  onMount(() => cancelButton?.focus());

  function onkeydown(event: KeyboardEvent) {
    // Whatever is pressed in here stays here: the clip browser and the window never see it.
    event.stopPropagation();
    handleKey(event);
  }

  /** The dialog's keys; the popover also sends it the keys pressed while the focus is outside it. */
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
    const controls = [...(dialog?.querySelectorAll<HTMLElement>("input, button") ?? [])];
    const at = controls.indexOf(document.activeElement as HTMLElement);
    // Out of the dialog, Tab enters at its first control and ⇧Tab at its last.
    const next = at < 0 ? (step > 0 ? 0 : controls.length - 1) : (at + step + controls.length) % controls.length;
    controls[next]?.focus();
  }
</script>

<div
  id="dialog-clear-history"
  class="alert"
  role="alertdialog"
  aria-modal="true"
  aria-labelledby="dialog-clear-title"
  aria-describedby="dialog-clear-desc"
  tabindex="-1"
  bind:this={dialog}
  {onkeydown}
>
  <div class="alert-icon"><AppIcon /></div>
  <h2 id="dialog-clear-title">Clear clipboard history?</h2>
  <p id="dialog-clear-desc">
    <span id="dialog-clear-count">{clipCountLabel(count)}</span>{CONSEQUENCE}
  </p>
  <label class="checkbox"
    ><input type="checkbox" id="chk-keep-pinned" bind:checked={keepPinned} /> Keep pinned clips and snippets</label
  >
  <div class="alert-buttons">
    <button type="button" class="btn btn-destructive-text" id="btn-clear-confirm" onclick={() => onconfirm(keepPinned)}
      >Clear history</button
    >
    <button type="button" class="btn btn-primary" id="btn-clear-cancel" bind:this={cancelButton} onclick={oncancel}
      >Cancel</button
    >
  </div>
</div>

<style>
  .alert {
    width: 260px;
    padding: 20px var(--space-16) var(--space-16);
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
    width: 56px;
    height: 56px;
    margin: 0 auto 10px;
  }

  h2 {
    margin: 0 0 6px;
    font-size: var(--text-body);
    line-height: var(--leading-body);
    font-weight: var(--weight-bold);
  }

  p {
    margin: 0 0 var(--space-12);
    font-size: var(--text-small);
    line-height: var(--leading-small);
  }

  .checkbox {
    display: inline-flex;
    align-items: center;
    gap: var(--space-6);
    margin: 0 0 14px;
    font-size: var(--text-small);
    text-align: left;
    cursor: default;
  }

  .checkbox input {
    appearance: none;
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    margin: 0;
    flex: none;
    border-radius: 3.5px;
    background: var(--color-control-bg);
    box-shadow: var(--shadow-control);
  }

  .checkbox input:checked {
    background: var(--color-accent)
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 14 14'%3E%3Cpath d='M3.5 7.2 6 9.7l4.6-5.4' fill='none' stroke='white' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E")
      center / 14px no-repeat;
    box-shadow: none;
  }

  .checkbox input:focus-visible,
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
    height: 28px;
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
    box-shadow: 0 0.5px 1.5px rgba(0, 0, 0, 0.2);
  }

  .btn-primary:active {
    background: var(--color-accent);
    filter: brightness(0.92);
  }
</style>
