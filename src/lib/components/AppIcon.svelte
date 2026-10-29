<script lang="ts">
  // The Tadidy app icon of the prototype (`appIconSvgInner`); each copy gets its own gradient id.
  const id = $props.id();
  const gradient = `app-icon-${id}`;
</script>

<svg class="app-icon" viewBox="0 0 96 96" aria-hidden="true">
  <defs>
    <linearGradient id={gradient} x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#5A8DFF" />
      <stop offset="1" stop-color="#3346D3" />
    </linearGradient>
  </defs>
  <rect x="6" y="6" width="84" height="84" rx="19" fill="url(#{gradient})" />
  <rect x="28" y="22" width="40" height="52" rx="7" fill="#fff" />
  <rect x="38" y="17" width="20" height="10" rx="4" fill="#E6ECFF" stroke="#3346D3" stroke-width="2" />
  <rect x="35" y="38" width="26" height="3.5" rx="1.75" fill="#BFCBF5" />
  <rect x="35" y="47" width="26" height="3.5" rx="1.75" fill="#BFCBF5" />
  <rect x="35" y="56" width="16" height="3.5" rx="1.75" fill="#BFCBF5" />
  <circle cx="64" cy="66" r="10" fill="#FF9F0A" stroke="#fff" stroke-width="3" />
  <path
    d="M64 61v5l3.2 2"
    fill="none"
    stroke="#fff"
    stroke-width="2.2"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
</svg>

<style>
  .app-icon {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
