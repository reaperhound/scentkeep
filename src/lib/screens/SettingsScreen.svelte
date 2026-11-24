<script lang="ts">
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import type { ParfumoStatus, WebdavBackup } from '$lib/backend';

  let {
    success, error, backupFolder, choosingBackupFolder, webdavBackup,
    webdavUrl = $bindable(), webdavUsername = $bindable(), webdavPassword = $bindable(),
    webdavBusy, parfumoStatus, parfumoLoginOpen, parfumoBusy,
    onChooseFolder, onSaveWebdav, onDisconnectWebdav, onOpenParfumo,
    onSaveParfumo, onDisconnectParfumo, onResetParfumo
  }: {
    success: string; error: string; backupFolder: string | null; choosingBackupFolder: boolean;
    webdavBackup: WebdavBackup | null; webdavUrl: string; webdavUsername: string; webdavPassword: string; webdavBusy: boolean;
    parfumoStatus: ParfumoStatus | null; parfumoLoginOpen: boolean; parfumoBusy: boolean;
    onChooseFolder: () => void; onSaveWebdav: (event: SubmitEvent) => void; onDisconnectWebdav: () => void;
    onOpenParfumo: () => void; onSaveParfumo: () => void; onDisconnectParfumo: () => void; onResetParfumo: () => void;
  } = $props();

  let darkMode = $state(false);
  const palettes = [
    { value: 'cds', label: 'Porcelain Blue', swatch: 'porcelain-swatch' },
    { value: 'huemint', label: 'Candy pop', swatch: '' },
    { value: 'velvet-dusk', label: 'Velvet Dusk', swatch: 'velvet-swatch' },
    { value: 'wildwood', label: 'Wildwood', swatch: 'wildwood-swatch' },
    { value: 'tropical-sorbet', label: 'Tropical Sorbet', swatch: 'tropical-swatch' },
    { value: 'amber-glow', label: 'Amber Glow', swatch: 'amber-swatch' },
    { value: 'crimson-ember', label: 'Crimson Ember', swatch: 'crimson-swatch' },
    { value: 'scarlet-bloom', label: 'Scarlet Bloom', swatch: 'scarlet-swatch' },
    { value: 'citrus-breeze', label: 'Citrus Breeze', swatch: 'citrus-swatch' },
    { value: 'miami-pop', label: 'Miami Pop', swatch: 'miami-swatch' },
    { value: 'velvet-saffron', label: 'Velvet Saffron', swatch: 'saffron-swatch' },
  ] as const;
  type Palette = (typeof palettes)[number]['value'];
  function isPalette(value: string | null | undefined): value is Palette {
    return palettes.some((option) => option.value === value);
  }
  let palette = $state<Palette>('cds');

  function readTheme() {
    const mode = document.documentElement.dataset.mode;
    darkMode = mode === 'dark' || (mode !== 'light' && window.matchMedia('(prefers-color-scheme: dark)').matches);
    const selectedPalette = document.documentElement.dataset.palette;
    palette = isPalette(selectedPalette) ? selectedPalette : 'cds';
  }

  function selectPalette(value: Palette) {
    palette = value;
    if (value !== 'cds') document.documentElement.dataset.palette = value;
    else delete document.documentElement.dataset.palette;
    try {
      localStorage.setItem('scentkeep.palette', value);
    } catch {
      // Keep the selected palette for this session if storage is unavailable.
    }
  }

  function toggleTheme() {
    darkMode = !darkMode;
    const value = darkMode ? 'dark' : 'light';
    document.documentElement.dataset.mode = value;
    try {
      localStorage.setItem('scentkeep.theme', value);
    } catch {
      // Keep the chosen theme for this session when storage is unavailable.
    }
  }

  onMount(() => {
    readTheme();
    const systemTheme = window.matchMedia('(prefers-color-scheme: dark)');
    function syncTheme(event: StorageEvent) {
      if (event.key === 'scentkeep.palette' || event.key === null) {
        if (isPalette(event.newValue) && event.newValue !== 'cds') document.documentElement.dataset.palette = event.newValue;
        else delete document.documentElement.dataset.palette;
        readTheme();
      }
      if (event.key !== 'scentkeep.theme' && event.key !== null) return;
      const value = event.newValue;
      if (value === 'light' || value === 'dark') document.documentElement.dataset.mode = value;
      else delete document.documentElement.dataset.mode;
      readTheme();
    }
    window.addEventListener('storage', syncTheme);
    systemTheme.addEventListener('change', readTheme);
    return () => {
      window.removeEventListener('storage', syncTheme);
      systemTheme.removeEventListener('change', readTheme);
    };
  });
</script>

<div class="page-content settings-page">
  <div class="heading"><div><h1>Settings</h1><p>Manage how Scentkeep keeps your collection safe.</p></div></div>
  {#if success}<div class="message success" role="status">{success}</div>{/if}
  {#if error}<div class="message error" role="alert">{error}</div>{/if}
  <section class="settings-card" aria-labelledby="appearance-settings-title">
    <div class="settings-card-header"><div><h2 id="appearance-settings-title">Appearance</h2><p>Choose your colors and switch between light and dark appearance.</p></div></div>
    <div class="palette-options" role="group" aria-label="Theme">
      {#each palettes as option (option.value)}
        <button class="palette-button" class:active={palette === option.value} type="button" aria-pressed={palette === option.value} onclick={() => selectPalette(option.value)}>
          <span class={`palette-swatch ${option.swatch}`} aria-hidden="true"><span></span><span></span><span></span><span></span></span>
          {option.label}
        </button>
      {/each}
    </div>
    <button class="theme-toggle" type="button" role="switch" aria-checked={darkMode} onclick={toggleTheme}>
      <span>Dark mode</span>
      <span class="theme-toggle-track" aria-hidden="true"><span class="theme-toggle-thumb"></span></span>
    </button>
  </section>
  <section class="settings-card" aria-labelledby="backup-settings-title">
    <div class="settings-card-header"><div><h2 id="backup-settings-title">Automatic backups</h2><p>Fragrances and notes are backed up as CSV files in the background after each change.</p></div><span class:configured={!!backupFolder} class="backup-status">{backupFolder ? 'On' : 'Not set'}</span></div>
    <div class="settings-field"><span>Backup folder</span><code>{backupFolder ?? 'No folder selected'}</code></div>
    <p class="settings-hint">{#if backupFolder}Files: scentkeep-fragrances.csv and scentkeep-notes.csv{:else}Choose a folder to start automatic backups.{/if}</p>
    <button class="primary-button" type="button" disabled={choosingBackupFolder || !isTauri()} onclick={onChooseFolder}>{choosingBackupFolder ? 'Choosing…' : backupFolder ? 'Change folder' : 'Choose folder'}</button>
  </section>
  <section class="settings-card webdav-card" aria-labelledby="webdav-settings-title">
    <div class="settings-card-header"><div><h2 id="webdav-settings-title">WebDAV backup</h2><p>Upload the same two CSV files to an HTTPS WebDAV folder in the background.</p></div><span class:configured={!!webdavBackup} class="backup-status">{webdavBackup ? 'Connected' : 'Not connected'}</span></div>
    {#if webdavBackup?.needsSync}<p class="settings-hint">This folder already has backups. Open your collection and select Sync from WebDAV before making changes; uploads are paused until then.</p>{/if}
    <form class="webdav-form" onsubmit={onSaveWebdav}>
      <label class="field">WebDAV folder URL<input type="url" required placeholder="https://cloud.example.com/remote.php/dav/files/you/Scentkeep/" bind:value={webdavUrl} disabled={webdavBusy || !isTauri()} /></label>
      <label class="field">Username<input required autocomplete="username" bind:value={webdavUsername} disabled={webdavBusy || !isTauri()} /></label>
      <label class="field">{webdavBackup ? 'New app password' : 'App password'}<input type="password" required autocomplete="new-password" bind:value={webdavPassword} disabled={webdavBusy || !isTauri()} /></label>
      <p class="settings-hint">Use an existing folder and an app password from your WebDAV provider. The password is stored in your system keyring.</p>
      <div class="parfumo-actions"><button class="primary-button" type="submit" disabled={webdavBusy || !isTauri()}>{webdavBusy ? 'Connecting…' : webdavBackup ? 'Update connection' : 'Connect WebDAV'}</button>{#if webdavBackup}<button class="parfumo-disconnect-button" type="button" disabled={webdavBusy} onclick={onDisconnectWebdav}>Disconnect</button>{/if}</div>
    </form>
  </section>
  <section class="settings-card parfumo-card" aria-labelledby="parfumo-settings-title">
    <div class="settings-card-header"><div><h2 id="parfumo-settings-title">Parfumo account</h2><p>Sign in on Parfumo and save the browser session to search its notes.</p></div><span class:configured={parfumoStatus?.connected} class="backup-status">{parfumoStatus?.connected ? 'Session saved' : 'Not connected'}</span></div>
    <p class="settings-hint">{#if parfumoLoginOpen}After signing in on Parfumo, return here and save the session.{:else if parfumoStatus?.connected}Your session is saved in the system keyring. Open Parfumo to refresh it, or disconnect to remove it.{:else}Open Parfumo and sign in there to connect your account.{/if}</p>
    <p class="settings-hint">If Cloudflare verification stays stuck, reset this browser's data and try again. If it still fails, try another network or contact Parfumo with the Ray ID shown on the page. You can still add notes by name.</p>
    <div class="parfumo-actions">
      <button class="primary-button" type="button" disabled={parfumoBusy || !isTauri()} onclick={onOpenParfumo}>{parfumoStatus?.connected ? 'Open Parfumo' : 'Connect Parfumo'}</button>
      {#if parfumoLoginOpen}<button class="parfumo-save-button" type="button" disabled={parfumoBusy} onclick={onSaveParfumo}>{parfumoStatus?.connected ? 'Update session' : 'Save session'}</button>{/if}
      {#if parfumoStatus?.connected}<button class="parfumo-disconnect-button" type="button" disabled={parfumoBusy} onclick={onDisconnectParfumo}>Disconnect</button>{/if}
      {#if parfumoStatus && !parfumoStatus.connected}<button class="back-button" type="button" disabled={parfumoBusy} onclick={onResetParfumo}>Reset browser data</button>{/if}
    </div>
  </section>
</div>
