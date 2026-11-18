<script lang="ts">
  import { tick } from 'svelte';
  import type { CsvImportReport, Fragrance, WebdavBackup } from '$lib/backend';
  import NavIcon from '$lib/NavIcon.svelte';

  let {
    entries,
    activeShelf,
    success,
    error,
    csvReport,
    csvBusy,
    webdavBackup,
    webdavSyncing,
    onSync,
    onImport,
    onExport,
    onAdd,
    onOpen,
    onEdit,
    onDelete
  }: {
    entries: Fragrance[];
    activeShelf: 'Collection' | 'Wishlist' | 'Empties';
    success: string;
    error: string;
    csvReport: { kind: 'fragrances' | 'notes'; result: CsvImportReport } | null;
    csvBusy: boolean;
    webdavBackup: WebdavBackup | null;
    webdavSyncing: boolean;
    onSync: () => void;
    onImport: (event: Event) => void;
    onExport: () => void;
    onAdd: () => void;
    onOpen: (item: Fragrance) => void;
    onEdit: (item: Fragrance) => void;
    onDelete: (item: Fragrance) => void;
  } = $props();

  const PAGE_SIZE = 12;
  let search = $state('');
  let filter = $state<'All' | 'Owned' | 'Wishlist' | 'Empties'>('All');
  let layout = $state<'grid' | 'list'>('grid');
  let page = $state(1);
  let openMenuId = $state<number | null>(null);
  let fileInput = $state<HTMLInputElement>();
  let listElement = $state<HTMLDivElement>();
  const categories = [
    { value: 'Women', icon: '/female.svg' },
    { value: 'Men', icon: '/male.svg' },
    { value: 'Unisex', icon: '/unisex-symbol-color-icon.svg' }
  ];
  const sexIconFor = (audience: string) => categories.find((item) => item.value === audience)?.icon ?? categories[2].icon;
  const visibleEntries = $derived(entries.filter((item) =>
    (activeShelf === 'Collection' ? (filter === 'All' || (filter === 'Owned' ? item.shelf === 'Collection' : item.shelf === filter)) : item.shelf === activeShelf) &&
    `${item.name} ${item.brand} ${item.collection}`.toLowerCase().includes(search.toLowerCase())
  ));
  const pageCount = $derived(Math.max(1, Math.ceil(visibleEntries.length / PAGE_SIZE)));
  const currentPage = $derived(Math.min(page, pageCount));
  const pagedEntries = $derived(visibleEntries.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE));

  $effect(() => {
    search;
    activeShelf;
    filter;
    page = 1;
  });

  async function changePage(nextPage: number) {
    page = nextPage;
    await tick();
    listElement?.scrollIntoView({
      behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
      block: 'start'
    });
  }
</script>

<div class="page-content collection-page">
  <div class="heading collection-heading"><div><h1>{activeShelf}</h1><p>A home for every scent worth remembering.</p></div></div>
  {#if success}<div class="message success">{success}</div>{/if}
  {#if error}<div class="message error">{error}</div>{/if}
  {#if webdavBackup?.needsSync}<div class="message import-result" role="status">This WebDAV folder already has backups. Sync them before making changes on this device.</div>{/if}
  {#if csvReport?.kind === 'fragrances'}
    <div class="message import-result" role="status"><strong>Imported {csvReport.result.imported} {csvReport.result.imported === 1 ? 'fragrance' : 'fragrances'}.</strong>{#if csvReport.result.duplicates.length}<p>Skipped {csvReport.result.duplicates.length} duplicate {csvReport.result.duplicates.length === 1 ? 'fragrance' : 'fragrances'}:</p><ul>{#each csvReport.result.duplicates as duplicate}<li>{duplicate}</li>{/each}</ul>{/if}</div>
  {/if}
  <div class="collection-controls">
    <div class="collection-search"><label class="search-box"><NavIcon name="search" size={18} /><input aria-label="Search fragrances" placeholder="Search fragrances or brands..." bind:value={search} /></label><button class="filter-button" type="button" aria-label={filter === 'Owned' ? 'Show all fragrances' : 'Show owned fragrances'} title={filter === 'Owned' ? 'Show all fragrances' : 'Show owned fragrances'} onclick={() => filter = filter === 'Owned' ? 'All' : 'Owned'}><NavIcon name="filter" size={18} /><span>Filters</span></button></div>
    <div class="csv-actions">{#if webdavBackup}<button class="back-button" type="button" disabled={webdavSyncing} onclick={onSync}><NavIcon name="refresh" size={18} /><span>{webdavSyncing ? 'Syncing…' : 'Sync from WebDAV'}</span></button>{/if}<button class="back-button" disabled={csvBusy} onclick={() => fileInput?.click()}><NavIcon name="import" size={18} /><span>Import CSV</span></button><button class="back-button" disabled={csvBusy} onclick={onExport}><NavIcon name="export" size={18} /><span>Export CSV</span></button><button class="primary-button" onclick={onAdd}><NavIcon name="add" size={18} /><span>Add fragrance</span></button><input type="file" accept=".csv,.xlsx,.xls,text/csv" bind:this={fileInput} onchange={onImport} hidden /></div>
  </div>
  <div class="collection-subbar">
    <div class="shelf-filters">{#if activeShelf === 'Collection'}{#each ['All', 'Owned', 'Wishlist', 'Empties'] as option}<button type="button" class:active={filter === option} onclick={() => filter = option as typeof filter}><NavIcon name={option === 'All' ? 'grid' : option === 'Owned' ? 'library' : option === 'Wishlist' ? 'wishlist' : 'empties'} size={18} /><span>{option}</span><span class="filter-count">{option === 'All' ? entries.length : option === 'Owned' ? entries.filter((item) => item.shelf === 'Collection').length : entries.filter((item) => item.shelf === option).length}</span></button>{/each}{/if}</div>
    <div class="collection-view"><span>{visibleEntries.length} {visibleEntries.length === 1 ? 'fragrance' : 'fragrances'}</span><div class="layout-switch"><button type="button" class:active={layout === 'grid'} aria-label="Grid view" onclick={() => layout = 'grid'}><NavIcon name="grid" size={18} /></button><button type="button" class:active={layout === 'list'} aria-label="List view" onclick={() => layout = 'list'}><NavIcon name="list" size={18} /></button></div></div>
  </div>
  {#if visibleEntries.length === 0}
    <div class="empty-state"><div><NavIcon name="bottle" size={48} /></div><h2>{entries.length === 0 ? 'Your collection is waiting' : 'No fragrances found'}</h2><p>{entries.length === 0 ? 'Add your first fragrance to start your scent journal.' : 'Try another search or filter.'}</p>{#if entries.length === 0}<button class="primary-button" onclick={onAdd}>Add your first fragrance <NavIcon name="forward" size={18} /></button>{/if}</div>
  {:else}
    <div class="fragrance-grid" class:list-layout={layout === 'list'} bind:this={listElement}>
      {#each pagedEntries as item}
        <div class="fragrance-card" role="link" tabindex="0" aria-label={`View details for ${item.name}`} onclick={() => onOpen(item)} onkeydown={(event) => { if (event.target === event.currentTarget && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); onOpen(item); } }}>
          <div class="fragrance-art">{#if item.imageUrl}<img src={item.imageUrl} alt={item.name} />{:else}<NavIcon name="bottle" size={72} />{/if}<div class="card-menu-wrap"><button class="card-menu" type="button" aria-label={`Actions for ${item.name}`} aria-expanded={openMenuId === item.id} onclick={(event) => { event.stopPropagation(); openMenuId = openMenuId === item.id ? null : (item.id ?? null); }}>···</button>{#if openMenuId === item.id}<div class="card-dropdown" role="menu" tabindex="-1" aria-label={`Actions for ${item.name}`} onclick={(event) => event.stopPropagation()} onkeydown={(event) => event.stopPropagation()}><button type="button" role="menuitem" onclick={() => { openMenuId = null; onEdit(item); }}><NavIcon name="edit" size={16} /> Edit</button><button type="button" role="menuitem" class="delete-action" onclick={() => { openMenuId = null; onDelete(item); }}><NavIcon name="trash" size={16} /> Delete</button></div>{/if}</div></div>
          <div class="fragrance-info"><h2>{item.name}</h2><div class="listing-meta"><img class="sex-icon" src={sexIconFor(item.audience)} alt={item.audience} title={item.audience} /><span class="concentration-label">{item.concentration || 'Concentration not set'}</span></div></div>
        </div>
      {/each}
    </div>
    {#if pageCount > 1}<nav class="pagination" aria-label="Fragrance pagination"><span>Showing {(currentPage - 1) * PAGE_SIZE + 1}–{Math.min(currentPage * PAGE_SIZE, visibleEntries.length)} of {visibleEntries.length}</span><div><button type="button" disabled={currentPage === 1} onclick={() => changePage(currentPage - 1)}>Previous</button><span aria-live="polite">Page {currentPage} of {pageCount}</span><button type="button" disabled={currentPage === pageCount} onclick={() => changePage(currentPage + 1)}>Next</button></div></nav>{/if}
  {/if}
</div>
