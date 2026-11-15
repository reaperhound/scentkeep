<script lang="ts">
  let { name }: { name: 'collection' | 'wishlist' | 'empties' | 'add' | 'notes' } = $props();
</script>

<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === 'collection'}
    <rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" /><rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" />
  {:else if name === 'wishlist'}
    <path d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8L12 21l8.8-8.6a5.5 5.5 0 0 0 0-7.8Z" />
  {:else if name === 'empties'}
    <path d="m7 7-3 3-2-3M4 10a8 8 0 0 1 14-5" /><path d="m17 17 3-3 2 3m-2-3a8 8 0 0 1-14 5" />
  {:else if name === 'add'}
    <circle cx="12" cy="12" r="9" /><path d="M12 8v8m-4-4h8" />
  {:else}
    <path d="M8 3h8m-7 0v4l-4 5v7a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-7l-4-5V3M5 12h14" />
  {/if}
</svg>
