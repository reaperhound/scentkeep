<script lang="ts">
  import type { AccordBar, Fragrance, NoteImage } from '$lib/backend';
  import AccordBars from '$lib/AccordBars.svelte';
  import NavIcon from '$lib/NavIcon.svelte';

  let { fragrance, noteImages, accords, accordsLoading, accordsError, onBack, onEdit }: {
    fragrance: Fragrance;
    noteImages: NoteImage[];
    accords: AccordBar[];
    accordsLoading: boolean;
    accordsError: string;
    onBack: () => void;
    onEdit: () => void;
  } = $props();

  const ratingFields = [{ key: 'scent', label: 'Scent' }, { key: 'longevity', label: 'Longevity' }, { key: 'sillage', label: 'Sillage' }, { key: 'bottle', label: 'Bottle' }, { key: 'valueForMoney', label: 'Value for money' }] as const;
  const imagesByName = $derived(new Map(noteImages.map((note) => [note.name.toLocaleLowerCase(), note.imageUrl])));
  function thumbnail(imageUrl: string, width = 50) {
    if (!imageUrl.startsWith('https://media.parfumo.com/notes_png/')) return imageUrl;
    const url = new URL(imageUrl);
    url.searchParams.set('width', String(width));
    url.searchParams.set('aspect_ratio', '1:1');
    return url.toString();
  }
</script>

<div class="page-content detail-page">
  <button class="detail-back" onclick={onBack}>← Back to collection</button>
  <div class="detail-layout">
    <div class="detail-left"><div class="detail-image">{#if fragrance.imageUrl}<img src={fragrance.imageUrl} alt={fragrance.name} />{:else}<div class="detail-placeholder"><NavIcon name="bottle" size={72} /><small>No bottle image</small></div>{/if}</div><div class="detail-image-caption">{fragrance.brand}, {fragrance.year}</div><button class="detail-edit" onclick={onEdit}>✎ Edit fragrance</button></div>
    <div class="detail-main">
      <h1>{fragrance.name}</h1><div class="detail-kicker">{fragrance.collection || 'Fragrance collection'}</div><div class="detail-brand">{fragrance.brand}<span class="year-pill">{fragrance.year}</span></div><div class="audience-pill">{fragrance.audience}</div>{#if fragrance.concentration}<div class="audience-pill concentration-pill">{fragrance.concentration}</div>{/if}
      <div class="ownership-summary"><span>{fragrance.shelf}</span><span>Owned: {fragrance.bottlesOwned}</span><span>Finished: {fragrance.bottlesFinished}</span>{#if fragrance.favourite}<span>★ Favourite</span>{/if}{#if fragrance.priceBought !== null}<span>Price bought: {fragrance.priceBought.toFixed(2)}</span>{/if}{#if fragrance.shelf === 'Wishlist' && fragrance.currentPrice !== null}<span>Current price: {fragrance.currentPrice.toFixed(2)}</span>{/if}</div>
      {#if fragrance.description}<p class="detail-description">{fragrance.description}</p>{/if}
      <AccordBars {accords} loading={accordsLoading} error={accordsError} />
      <section class="detail-section"><div class="detail-section-title"><h2>Fragrance pyramid</h2></div>
        {#each [{ title: 'Top notes', notes: fragrance.topNotes }, { title: 'Heart notes', notes: fragrance.heartNotes }, { title: 'Base notes', notes: fragrance.baseNotes }] as group}
          <div class="pyramid-row"><div class="pyramid-label">{group.title}</div><div class="pyramid-notes">{#each group.notes as note}{@const image = imagesByName.get(note.toLocaleLowerCase())}<div class="pyramid-note"><div class="pyramid-note-image">{#if image}<img src={thumbnail(image)} alt="" />{:else}<NavIcon name="notes" size={18} />{/if}</div><span>{note}</span></div>{/each}{#if group.notes.length === 0}<span class="no-notes">No notes recorded</span>{/if}</div></div>
        {/each}
      </section>
      <div class="detail-meta"><section><div class="detail-section-title"><h2>Perfumer</h2></div><p>{fragrance.perfumer || 'Not recorded'}</p></section><section><div class="detail-section-title"><h2>Overall rating</h2></div>{#if fragrance.bottleRating !== null}<div class="detail-rating"><strong>{fragrance.bottleRating.toFixed(1)}</strong><span>/ 10</span></div><div class="rating-bar"><div style={`width: ${fragrance.bottleRating * 10}%`}></div></div>{#if fragrance.ratings && ratingFields.some(({ key }) => fragrance.ratings[key] !== null)}<div class="rating-breakdown">{#each ratingFields as { key, label }}{#if fragrance.ratings[key] !== null}<small>{label}: {fragrance.ratings[key]?.toFixed(1)} / 10</small>{/if}{/each}</div>{/if}{:else}<p>Not rated yet</p>{/if}</section></div>
    </div>
  </div>
</div>
