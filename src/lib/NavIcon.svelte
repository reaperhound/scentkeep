<script lang="ts">
  type Icon = 'collection' | 'wishlist' | 'empties' | 'add' | 'notes' | 'search' | 'grid' | 'list' | 'library' | 'filter' | 'menu' | 'back' | 'forward' | 'refresh' | 'more' | 'edit' | 'trash' | 'bottle' | 'import' | 'export' | 'female' | 'male' | 'unisex';
  let { name, size = 20 }: { name: Icon; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === 'collection' || name === 'grid'}
    <rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" /><rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" />
  {:else if name === 'wishlist'}
    <path d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8L12 21l8.8-8.6a5.5 5.5 0 0 0 0-7.8Z" />
  {:else if name === 'empties'}
    <rect x="4" y="5" width="16" height="4" rx="1" /><path d="M6 9v10a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V9m-8 4h4" />
  {:else if name === 'add'}
    <circle cx="12" cy="12" r="9" /><path d="M12 8v8m-4-4h8" />
  {:else if name === 'notes'}
    <path d="M20 4c-9 0-15 4-15 11a5 5 0 0 0 5 5c7 0 11-6 10-16Z" /><path d="M5 20c2-4 6-7 11-10" />
  {:else if name === 'search'}
    <circle cx="10.8" cy="10.8" r="6.8" /><path d="m16 16 4.5 4.5" />
  {:else if name === 'list'}
    <path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" />
  {:else if name === 'library'}
    <path d="M4 3v18m5-18v18m7-16 4 15M2 21h20" />
  {:else if name === 'filter'}
    <path d="M4 7h16M7 12h10m-7 5h4" />
  {:else if name === 'menu'}
    <path d="M4 6h16M4 12h16M4 18h16" />
  {:else if name === 'back'}
    <path d="m12 19-7-7 7-7M5 12h14" />
  {:else if name === 'forward'}
    <path d="m12 5 7 7-7 7M5 12h14" />
  {:else if name === 'refresh'}
    <path d="M20 7v5h-5M4 17v-5h5" /><path d="M5.5 9A7.5 7.5 0 0 1 18 6l2 1M4 17l2 1a7.5 7.5 0 0 0 12.5-3" />
  {:else if name === 'more'}
    <circle cx="5" cy="12" r="1" /><circle cx="12" cy="12" r="1" /><circle cx="19" cy="12" r="1" />
  {:else if name === 'edit'}
    <path d="M12 20h9M16 4a2.1 2.1 0 0 1 3 3L8 18l-4 1 1-4L16 4Z" />
  {:else if name === 'trash'}
    <path d="M3 6h18m-2 0-1 14H6L5 6m3 0V4h8v2m-6 4v7m4-7v7" />
  {:else if name === 'bottle'}
    <path d="M9 3h6m-5 0v4l-4 4v8a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2v-8l-4-4V3M6 12h12" />
  {:else if name === 'import'}
    <path d="M12 3v12m-4-4 4 4 4-4M4 17v4h16v-4" />
  {:else if name === 'export'}
    <path d="M12 16V4m-4 4 4-4 4 4M4 17v4h16v-4" />
  {:else if name === 'female'}
    <circle cx="12" cy="9" r="6" /><path d="M12 15v7m-4-3h8" />
  {:else if name === 'male'}
    <circle cx="9" cy="15" r="6" /><path d="m13.5 10.5 7-7M15 3.5h5.5V9" />
  {:else}
    <circle cx="12" cy="12" r="8" /><path d="M12 4v16M4 12h16" />
  {/if}
</svg>
