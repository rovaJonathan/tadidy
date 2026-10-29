<script lang="ts">
  type Props = { variant: "history" } | { variant: "search"; query: string };

  let props: Props = $props();
</script>

{#if props.variant === "history"}
  <div id="empty-history" class="empty-state" role="status">
    <div class="empty-icon">
      <svg class="ico" viewBox="0 0 16 16" aria-hidden="true"
        ><path
          d="M5.5 3H4.5A1.5 1.5 0 0 0 3 4.5v8A1.5 1.5 0 0 0 4.5 14h7a1.5 1.5 0 0 0 1.5-1.5v-8A1.5 1.5 0 0 0 11.5 3h-1"
        /><rect x="5.5" y="1.75" width="5" height="2.5" rx="1" /></svg
      >
    </div>
    <p class="empty-title">Nothing copied yet.</p>
    <p class="empty-body">Press ⌘C anywhere — Tadidy remembers.</p>
  </div>
{:else}
  <div id="empty-search" class="empty-state" role="status">
    <div class="empty-icon">
      <svg class="ico" viewBox="0 0 16 16" aria-hidden="true"
        ><circle cx="7" cy="7" r="4.25" /><path d="M10.2 10.2 13.5 13.5" /></svg
      >
    </div>
    <p class="empty-title">No clips match “<span id="empty-search-query">{props.query}</span>”</p>
    <p class="empty-body">Check the spelling, or try another tab or filter.</p>
  </div>
{/if}

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
</style>
