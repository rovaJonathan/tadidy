<script lang="ts">
  import { onMount } from "svelte";
  import { checkAccessibility, onAccessibilityChanged, openAccessibilitySettings } from "../api";

  // Unknown until the core answers: a banner that flashed for a granted permission would be worse
  // than one that comes a moment late.
  let trusted = $state<boolean | undefined>();

  onMount(() => {
    let removed = false;
    // An event is newer than a check that was asked before it arrived.
    let eventSeen = false;
    let unlisten: (() => void) | undefined;

    void onAccessibilityChanged((change) => {
      eventSeen = true;
      trusted = change.trusted;
    }).then((stop) => (removed ? stop() : (unlisten = stop)));
    void checkAccessibility().then((granted) => {
      if (!eventSeen) trusted = granted;
    });

    return () => {
      removed = true;
      unlisten?.();
    };
  });

  async function enable() {
    try {
      await openAccessibilitySettings();
    } catch {
      // The banner stays: the user can click again. There is no toast to tell them yet.
    }
  }
</script>

<div id="banner-accessibility" role="status" hidden={trusted !== false}>
  <svg class="ico" viewBox="0 0 16 16" aria-hidden="true"
    ><path d="M8 2.2 14.2 13H1.8z" fill="currentColor" stroke="currentColor" stroke-width="1.6" /><path
      d="M8 6.3v3.2"
      stroke="#fff"
      stroke-width="1.5"
    /><circle cx="8" cy="11.3" r=".85" fill="#fff" stroke="none" /></svg
  >
  <span class="banner-text">Auto-paste disabled — clips are copied only.</span>
  <button type="button" id="btn-enable-accessibility" class="btn btn-small" onclick={enable}>Enable</button>
</div>

<style>
  #banner-accessibility {
    grid-area: banner;
    display: flex;
    align-items: center;
    gap: var(--space-8);
    margin: 0 var(--space-8) var(--space-8);
    padding: 5px 5px 5px var(--space-10);
    border-radius: var(--radius-lg);
    background: var(--color-warning-bg);
    font-size: var(--text-callout);
    line-height: var(--leading-callout);
  }

  /* `display: flex` above would win over the `hidden` attribute. */
  #banner-accessibility[hidden] {
    display: none;
  }

  .ico {
    flex: none;
    color: var(--color-warning);
    width: 15px;
    height: 15px;
  }

  .banner-text {
    flex: 1;
    min-width: 0;
  }

  .btn {
    height: 20px;
    padding: 0 var(--space-8);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--color-control-bg);
    box-shadow: var(--shadow-control);
    font: inherit;
    font-size: var(--text-callout);
    color: var(--color-text-primary);
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .btn:active {
    background: var(--color-fill-pressed);
  }

  .btn:focus-visible {
    outline: 3px solid var(--color-focus-ring);
  }
</style>
