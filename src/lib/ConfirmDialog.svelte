<script lang="ts">
  import type { Action, StartupItem } from './types';
  import { CATEGORY_LABEL } from './labels';

  interface Props {
    item: StartupItem;
    action: Action;
    busy: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  }

  let { item, action, busy, onconfirm, oncancel }: Props = $props();

  const verb = $derived(action === 'delete' ? 'Delete' : action === 'disable' ? 'Disable' : 'Enable');
  const titleId = 'confirm-title';

  let cancelButton = $state<HTMLButtonElement | null>(null);

  $effect(() => {
    cancelButton?.focus();
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && !busy) oncancel();
  }
</script>

<svelte:window {onkeydown} />

<div class="backdrop">
  <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby={titleId} aria-describedby="confirm-body">
    <h2 id={titleId}>{verb} “{item.display_name}”?</h2>
    <p id="confirm-body">
      {#if item.category === 'critical'}
        <span class="badge critical">{CATEGORY_LABEL[item.category]}</span>
        This item is flagged as Critical. {item.explanation}
      {:else if action === 'delete'}
        Deleting removes the entry permanently. Disabling is usually enough and can be undone.
      {:else}
        This change can be undone from this list.
      {/if}
    </p>
    {#if action === 'delete'}
      <p class="warn">The entry will be removed from <code>{item.location}</code>.</p>
    {/if}
    <div class="actions">
      <button bind:this={cancelButton} type="button" class="secondary" onclick={oncancel} disabled={busy}>
        Cancel
      </button>
      <button
        type="button"
        class={action === 'enable' ? 'primary' : 'danger'}
        onclick={onconfirm}
        disabled={busy}
      >
        {busy ? 'Working…' : `${verb} item`}
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    display: grid;
    place-items: center;
    padding: 16px;
    z-index: 10;
  }
  .dialog {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    max-width: 460px;
    width: 100%;
    padding: 20px;
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.3);
  }
  h2 {
    font-size: 1.15rem;
    margin: 0 0 8px;
    overflow-wrap: anywhere;
  }
  p {
    margin: 8px 0;
  }
  .warn {
    color: var(--danger);
  }
  code {
    overflow-wrap: anywhere;
    font-size: 0.85em;
  }
  .badge.critical {
    background: var(--crit-bg);
    color: var(--crit);
    padding: 1px 8px;
    border-radius: 999px;
    font-size: 0.85em;
    font-weight: 600;
    margin-right: 4px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  button {
    padding: 8px 14px;
    border-radius: 6px;
    border: 1px solid var(--border);
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .secondary {
    background: var(--surface-2);
  }
  .primary {
    background: var(--accent);
    color: var(--accent-text);
    border-color: var(--accent);
  }
  .danger {
    background: var(--danger);
    color: #fff;
    border-color: var(--danger);
  }
</style>
