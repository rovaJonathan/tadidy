<script lang="ts">
  interface Props {
    count: string;
    /** The ⚙ button was clicked. */
    onpreferences?: () => void;
  }

  let { count, onpreferences }: Props = $props();
</script>

<footer class="browser-footer">
  <span id="clip-count">{count}</span>
  <!-- Only the actions of the MVP: Plain text and Actions come with their P1 tickets. -->
  <span class="footer-hints" aria-hidden="true">
    <span><b>↩</b> Paste</span><span><b>⌥↩</b> Copy</span>
  </span>
  <button
    type="button"
    id="btn-preferences"
    class="icon-btn"
    aria-label="Preferences"
    title="Preferences (⌘,)"
    onclick={() => onpreferences?.()}
  >
    <svg class="ico" viewBox="0 0 16 16" aria-hidden="true"
      ><circle cx="8" cy="8" r="5.3" stroke-width="2.6" stroke-dasharray="2.08 2.08" stroke-linecap="butt" /><circle
        cx="8"
        cy="8"
        r="4.1"
      /><circle cx="8" cy="8" r="1.7" /></svg
    >
  </button>
</footer>

<style>
  .browser-footer {
    grid-area: footer;
    display: flex;
    align-items: center;
    gap: var(--space-8);
    height: 32px;
    padding: 0 var(--space-6) 0 var(--space-12);
    border-top: 0.5px solid var(--color-separator);
    font-size: var(--text-small);
    color: var(--color-text-secondary);
  }

  #clip-count {
    font-variant-numeric: tabular-nums;
  }

  .footer-hints {
    flex: 1;
    display: flex;
    justify-content: center;
    gap: var(--space-12);
    color: var(--color-text-tertiary);
  }

  .footer-hints span {
    display: inline-flex;
    gap: 4px;
    align-items: center;
  }

  .footer-hints b {
    font-weight: var(--weight-medium);
    color: var(--color-text-secondary);
  }

  /* The popover is narrow: the hints take the space but stay invisible, as in the prototype. */
  :global([data-layout="popover"]) .footer-hints {
    visibility: hidden;
  }

  .icon-btn {
    width: 24px;
    height: 24px;
    margin-left: auto;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    display: grid;
    place-items: center;
    padding: 0;
    color: var(--color-text-secondary);
  }

  .icon-btn:hover {
    background: var(--color-fill-hover);
    color: var(--color-text-primary);
  }

  .icon-btn:active {
    background: var(--color-fill-pressed);
  }

  .icon-btn:focus-visible {
    outline: 3px solid var(--color-focus-ring);
  }

  .ico {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
