<script lang="ts">
  import type { AccordBar } from '$lib/backend';
  import './accord-bars.css';

  let {
    accords,
    loading = false,
    error = ''
  }: { accords: AccordBar[]; loading?: boolean; error?: string } = $props();
</script>

<section class="detail-section accord-section" aria-labelledby="main-accords-title">
  <header class="accord-heading">
    <h2 id="main-accords-title">Main accords</h2>
    <p id="main-accords-description">Relative strength estimated from the categories assigned to each note.</p>
  </header>

  {#if loading}
    <div class="accord-state" role="status">
      <span>Calculating accords…</span>
      <div class="accord-skeleton" aria-hidden="true"><i></i><i></i><i></i></div>
    </div>
  {:else if error}
    <p class="accord-state accord-state-error" role="alert">Could not calculate accords: {error}</p>
  {:else if accords.length}
    <ol class="accord-list" aria-describedby="main-accords-description">
      {#each accords as accord}
        <li class="accord-row" title={`Contributing notes: ${accord.contributingNotes.join(', ')}`}>
          <div
            class="accord-meter"
            role="meter"
            aria-label={accord.label}
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={accord.relativeStrength}
            aria-valuetext={`${accord.relativeStrength}% relative strength`}
          >
            <span class="accord-fill" style={`--accord-value: ${accord.relativeStrength}%`} aria-hidden="true"></span>
            <span class="accord-name">{accord.label}</span>
          </div>
          <output aria-label={`${accord.label} relative strength`}>{accord.relativeStrength}%</output>
        </li>
      {/each}
    </ol>
  {:else}
    <p class="accord-state">Add categories to this fragrance’s notes to generate its accord profile.</p>
  {/if}
</section>
