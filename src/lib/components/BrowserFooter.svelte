<script lang="ts">
  let { count }: { count: string } = $props();
</script>

<footer class="browser-footer">
  <span id="clip-count">{count}</span>
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
</style>
