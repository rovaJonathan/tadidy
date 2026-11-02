<script lang="ts">
  interface Props {
    /** Reset history… was clicked: the host opens the confirmation. */
    onreset: () => void;
  }

  let { onreset }: Props = $props();
</script>

<div id="storage-unavailable" class="empty-state" role="status">
  <div class="empty-icon">
    <svg class="ico" viewBox="0 0 16 16" aria-hidden="true"
      ><path d="M8 2.2 14 12.8H2L8 2.2Z" /><path d="M8 6.6v2.7" /><circle cx="8" cy="11.1" r="0.35" /></svg
    >
  </div>
  <p class="empty-title">Your history can’t be opened.</p>
  <p class="empty-body">Tadidy can’t unlock its encrypted database, so nothing is being recorded.</p>
  <button type="button" class="btn" id="btn-reset-history" onclick={onreset}>Reset history…</button>
</div>

<style>
  .empty-state {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-6);
    padding: var(--space-24);
    text-align: center;
    color: var(--color-text-secondary);
  }

  .empty-icon {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: var(--color-fill);
    display: grid;
    place-items: center;
    color: var(--color-text-tertiary);
    margin-bottom: var(--space-4);
  }

  .ico {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  p {
    margin: 0;
    max-width: 260px;
  }

  .empty-title {
    font-size: var(--text-headline);
    line-height: var(--leading-headline);
    font-weight: var(--weight-semibold);
    color: var(--color-text-primary);
  }

  .btn {
    height: 28px;
    margin-top: var(--space-8);
    padding: 0 var(--space-12);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--color-control-bg);
    box-shadow: var(--shadow-control);
    font: inherit;
    font-size: var(--text-body);
    color: var(--color-text-primary);
    white-space: nowrap;
  }

  .btn:active {
    background: var(--color-fill-pressed);
  }

  .btn:focus-visible {
    outline: 3px solid var(--color-focus-ring);
  }
</style>
