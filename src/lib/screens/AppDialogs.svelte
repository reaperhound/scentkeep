<script lang="ts">
  let {
    profileOpen = $bindable(), profileName = $bindable(), profilePicture = $bindable(), profilePictureUrl = $bindable(),
    backupOpen = $bindable(), backupFolder, choosingBackupFolder, backupError,
    closeBackupOpen = $bindable(), onSaveProfile, onChooseBackupFolder, onShowWebdav, onCloseWithPendingBackup
  }: {
    profileOpen: boolean; profileName: string; profilePicture: string; profilePictureUrl: string;
    backupOpen: boolean; backupFolder: string | null; choosingBackupFolder: boolean; backupError: string;
    closeBackupOpen: boolean; onSaveProfile: () => void; onChooseBackupFolder: () => void;
    onShowWebdav: () => void; onCloseWithPendingBackup: () => void;
  } = $props();
</script>

{#if profileOpen}
  <div class="dialog-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) profileOpen = false; }}>
    <dialog open class="profile-dialog" aria-labelledby="profile-title">
      <button class="dialog-close" type="button" aria-label="Close" onclick={() => profileOpen = false}>×</button>
      <h2 id="profile-title">Edit profile</h2>
      <div class="profile-picture-preview">{#if profilePicture}<img src={profilePicture} alt="Profile preview" />{:else}<span>{profileName.trim().charAt(0).toUpperCase() || 'S'}</span>{/if}</div>
      <label class="field"><span class="field-label-row"><span class="field-label-text">Your name</span></span><input maxlength="60" bind:value={profileName} placeholder="Your name" /></label>
      <div class="profile-url-row"><input type="url" aria-label="Profile picture URL" placeholder="Or paste an image URL" bind:value={profilePictureUrl} /><button type="button" disabled={!profilePictureUrl.trim()} onclick={() => profilePicture = profilePictureUrl.trim()}>Use link</button></div>
      {#if profilePicture}<button type="button" class="clear-picture" onclick={() => profilePicture = ''}>Remove picture</button>{/if}
      <div class="dialog-actions"><button class="back-button" type="button" onclick={() => profileOpen = false}>Cancel</button><button class="primary-button" type="button" onclick={onSaveProfile}>Save profile</button></div>
    </dialog>
  </div>
{/if}

{#if backupOpen}
  <div class="dialog-backdrop" role="presentation"><dialog open class="profile-dialog" aria-labelledby="backup-title">
    <div class="eyebrow">AUTOMATIC BACKUPS</div><h2 id="backup-title">Set up automatic backups</h2>
    <p>After every change, Scentkeep backs up your fragrances and notes as two CSV files in a local folder or WebDAV folder without delaying your save.</p>
    {#if backupFolder}<p class="backup-path">Current folder: {backupFolder}</p>{/if}
    {#if backupError}<div class="message error" role="alert">{backupError}</div>{/if}
    <div class="dialog-actions"><button class="back-button" type="button" onclick={() => backupOpen = false}>Later</button><button class="back-button" type="button" onclick={onShowWebdav}>Set up WebDAV</button><button class="primary-button" type="button" disabled={choosingBackupFolder} onclick={onChooseBackupFolder}>{choosingBackupFolder ? 'Choosing…' : 'Choose folder'}</button></div>
  </dialog></div>
{/if}

{#if closeBackupOpen}
  <div class="dialog-backdrop" role="presentation"><dialog open class="profile-dialog" aria-labelledby="backup-close-title">
    <h2 id="backup-close-title">Backup still running</h2><p>Your changes are saved locally. Wait for the backup to finish, or quit now and retry it when Scentkeep next opens.</p>
    <div class="dialog-actions"><button class="back-button" type="button" onclick={() => closeBackupOpen = false}>Wait for backup</button><button class="primary-button" type="button" onclick={onCloseWithPendingBackup}>Quit and retry later</button></div>
  </dialog></div>
{/if}
