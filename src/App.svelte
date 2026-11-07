<script lang="ts">
  import { onMount } from 'svelte';
  import { applyAction, auditLog, listItems } from './lib/api';
  import type { Action, AuditEntry, Category, ScanResult, StartupItem } from './lib/types';
  import { CATEGORY_LABEL, IMPACT_LABEL, SOURCE_LABEL, needsConfirmation } from './lib/labels';
  import ConfirmDialog from './lib/ConfirmDialog.svelte';
  import Glossary from './lib/Glossary.svelte';

  type Tab = 'all' | Category;
  const TABS: { value: Tab; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'safe', label: 'Safe' },
    { value: 'recommended', label: 'Recommended' },
    { value: 'critical', label: 'Critical' },
  ];

  let items = $state<StartupItem[]>([]);
  let warnings = $state<string[]>([]);
  let audit = $state<AuditEntry[]>([]);
  let loading = $state(true);
  let scanError = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let busy = $state(false);

  // First-run notice. Per-viewer convenience only, so storage failures are ignored.
  const INTRO_KEY = 'tpt-startup.intro-dismissed';
  function introDismissed(): boolean {
    try {
      return localStorage.getItem(INTRO_KEY) === '1';
    } catch {
      return false;
    }
  }
  let showIntro = $state(!introDismissed());
  function dismissIntro() {
    showIntro = false;
    try {
      localStorage.setItem(INTRO_KEY, '1');
    } catch {
      // Storage unavailable: the notice simply shows again next time.
    }
  }

  let tab = $state<Tab>('all');
  let query = $state('');
  let selectedId = $state<string | null>(null);
  let pending = $state<{ item: StartupItem; action: Action } | null>(null);

  const counts = $derived({
    all: items.length,
    safe: items.filter((i) => i.category === 'safe').length,
    recommended: items.filter((i) => i.category === 'recommended').length,
    critical: items.filter((i) => i.category === 'critical').length,
  });

  const visible = $derived(
    items.filter((i) => {
      if (tab !== 'all' && i.category !== tab) return false;
      if (!query.trim()) return true;
      const q = query.trim().toLowerCase();
      return [i.display_name, i.publisher ?? '', i.command ?? '', SOURCE_LABEL[i.source]].some((s) =>
        s.toLowerCase().includes(q),
      );
    }),
  );

  const selected = $derived(items.find((i) => i.id === selectedId) ?? null);

  async function refresh() {
    loading = true;
    scanError = null;
    try {
      const result: ScanResult = await listItems();
      items = result.items;
      warnings = result.warnings;
      audit = await auditLog();
      if (selectedId && !items.some((i) => i.id === selectedId)) selectedId = null;
    } catch (e) {
      scanError = String(e);
    } finally {
      loading = false;
    }
  }

  function requestAction(item: StartupItem, action: Action) {
    notice = null;
    if (needsConfirmation(item.category, action)) {
      pending = { item, action };
    } else {
      void perform(item, action, false);
    }
  }

  async function perform(item: StartupItem, action: Action, confirmed: boolean) {
    busy = true;
    try {
      await applyAction(item.id, action, confirmed);
      const done = action === 'delete' ? 'deleted' : action === 'disable' ? 'disabled' : 'enabled';
      notice = `“${item.display_name}” ${done}.`;
      if (action === 'delete' && selectedId === item.id) selectedId = null;
      pending = null;
      await refresh();
    } catch (e) {
      const message = String(e);
      const hint = message.includes('permission denied')
        ? ' This entry is system-wide. Close tpt-startup and reopen it as administrator to change it.'
        : '';
      notice = `Could not change “${item.display_name}”: ${message}.${hint}`;
      pending = null;
    } finally {
      busy = false;
    }
  }

  function actionsFor(item: StartupItem): Action[] {
    const toggle: Action[] = item.reversible_disable ? [item.enabled ? 'disable' : 'enable'] : [];
    // systemd units are turned off, never removed, by this app (see platform-linux/src/systemd.rs).
    if (item.source === 'systemd-unit') return toggle;
    return [...toggle, 'delete'];
  }

  function actionLabel(a: Action): string {
    return a === 'enable' ? 'Enable' : a === 'disable' ? 'Disable' : 'Delete';
  }

  function formatTime(ts: number): string {
    return new Date(ts * 1000).toLocaleTimeString();
  }

  onMount(refresh);
</script>

<header class="topbar">
  <div class="brand">
    <strong>tpt-startup</strong>
    <span class="muted">Startup items</span>
  </div>
  <button type="button" class="secondary" onclick={refresh} disabled={loading}>
    {loading ? 'Scanning…' : 'Refresh'}
  </button>
</header>

<main>
  {#if showIntro}
    <section class="banner intro" aria-labelledby="intro-title">
      <div>
        <h2 id="intro-title">Welcome to tpt-startup</h2>
        <p>
          This lists programs that start with Windows or Linux. Each one is labelled
          <strong>Safe</strong>, <strong>Recommended</strong> or <strong>Critical</strong>
          with a plain explanation. Turning an item off is reversible; deleting it is not.
          Changes to system-wide entries need administrator rights.
        </p>
      </div>
      <button type="button" class="secondary" onclick={dismissIntro}>Got it</button>
    </section>
  {/if}

  {#if notice}
    <div class="banner" role="status">
      <span>{notice}</span>
      <button type="button" class="link" onclick={() => (notice = null)}>Dismiss</button>
    </div>
  {/if}

  {#if warnings.length}
    <div class="banner warn" role="status">
      <strong>Some sources could not be read:</strong>
      <ul>
        {#each warnings as w}<li>{w}</li>{/each}
      </ul>
    </div>
  {/if}

  <div class="toolbar">
    <div class="tabs" role="tablist" aria-label="Filter by category">
      {#each TABS as t}
        <button
          type="button"
          role="tab"
          aria-selected={tab === t.value}
          class:active={tab === t.value}
          onclick={() => (tab = t.value)}
        >
          {t.label} <span class="count">{counts[t.value]}</span>
        </button>
      {/each}
    </div>
    <label class="search">
      <span class="sr-only">Search startup items</span>
      <input type="search" placeholder="Search name, publisher, command…" bind:value={query} />
    </label>
  </div>

  <div class="layout">
    <section class="list" aria-label="Startup items">
      {#if loading && items.length === 0}
        <p class="state">Scanning startup items…</p>
      {:else if scanError}
        <div class="state error">
          <p>Scan failed: {scanError}</p>
          <button type="button" onclick={refresh}>Try again</button>
        </div>
      {:else if items.length === 0}
        <p class="state">No startup items found for your account.</p>
      {:else if visible.length === 0}
        <p class="state">Nothing matches this filter.</p>
      {:else}
        <ul>
          {#each visible as item (item.id)}
            <li class:selected={item.id === selectedId}>
              <button
                type="button"
                class="row"
                onclick={() => (selectedId = item.id)}
                aria-current={item.id === selectedId}
              >
                <span class="row-main">
                  <span class="name">{item.display_name}</span>
                  <span class="muted small"
                    >{SOURCE_LABEL[item.source]}{item.publisher ? ` · ${item.publisher}` : ''}</span
                  >
                </span>
                <span class="badge {item.category}">{CATEGORY_LABEL[item.category]}</span>
                <span class="state-pill" class:off={!item.enabled}>{item.enabled ? 'On' : 'Off'}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <aside class="detail" aria-label="Item details">
      {#if selected}
        <h2>{selected.display_name}</h2>
        <p><span class="badge {selected.category}">{CATEGORY_LABEL[selected.category]}</span></p>
        <p class="explain">{selected.explanation}</p>
        <dl>
          <dt>Where it starts</dt><dd>{SOURCE_LABEL[selected.source]}</dd>
          <dt>Publisher</dt><dd>{selected.publisher ?? 'Unknown'}</dd>
          <dt>Boot impact</dt><dd>{IMPACT_LABEL[selected.boot_impact]}</dd>
          <dt>Status</dt><dd>{selected.enabled ? 'Enabled' : 'Disabled'}</dd>
          <dt>Command</dt><dd><code>{selected.command ?? 'Not available'}</code></dd>
          <dt>Location</dt><dd><code>{selected.location}</code></dd>
        </dl>
        <div class="actions">
          {#each actionsFor(selected) as a}
            <button
              type="button"
              class={a === 'delete' ? 'danger' : a === 'enable' ? 'primary' : 'secondary'}
              onclick={() => requestAction(selected, a)}
              disabled={busy}
            >
              {actionLabel(a)}
            </button>
          {/each}
        </div>
        {#if !selected.reversible_disable}
          <p class="muted small">This source cannot be turned off without deleting it.</p>
        {:else if selected.source === 'systemd-unit'}
          <p class="muted small">systemd units can be turned off or on here, but not deleted.</p>
        {/if}
      {:else}
        <p class="state">Select an item to see what it does and whether it is safe to change.</p>
      {/if}

      <details class="audit">
        <summary>Recent changes ({audit.length})</summary>
        {#if audit.length === 0}
          <p class="muted small">No changes this session.</p>
        {:else}
          <ul>
            {#each [...audit].reverse() as entry}
              <li class="small">
                {formatTime(entry.timestamp)} · {actionLabel(entry.action)} “{entry.display_name}”
                {#if entry.error}<span class="error-text">failed: {entry.error}</span>{/if}
              </li>
            {/each}
          </ul>
        {/if}
      </details>

      <Glossary />
    </aside>
  </div>
</main>

{#if pending}
  <ConfirmDialog
    item={pending.item}
    action={pending.action}
    {busy}
    onconfirm={() => pending && perform(pending.item, pending.action, true)}
    oncancel={() => (pending = null)}
  />
{/if}

<style>
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    z-index: 5;
  }
  .brand {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  main {
    padding: 16px;
    max-width: 1200px;
    margin: 0 auto;
  }
  .muted {
    color: var(--muted);
  }
  .small {
    font-size: 0.85rem;
  }
  .banner {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    margin-bottom: 12px;
  }
  .banner.intro h2 {
    font-size: 1rem;
    margin: 0 0 4px;
  }
  .banner.intro p {
    margin: 0;
    color: var(--muted);
    font-size: 0.9rem;
  }
  .banner.warn ul {
    margin: 4px 0 0;
    padding-left: 18px;
  }
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tabs button {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 6px 12px;
    cursor: pointer;
  }
  .tabs button.active {
    background: var(--accent);
    color: var(--accent-text);
    border-color: var(--accent);
  }
  .count {
    opacity: 0.75;
    font-size: 0.85em;
  }
  .search input {
    width: min(320px, 100%);
    padding: 7px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface);
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  @media (max-width: 760px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
  .list,
  .detail {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 8px;
  }
  .list ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .list li + li {
    border-top: 1px solid var(--border);
  }
  .list li.selected {
    background: var(--surface-2);
  }
  .row {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 10px;
    align-items: center;
    text-align: left;
    background: none;
    border: 0;
    padding: 10px;
    cursor: pointer;
  }
  .row-main {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 0.8rem;
    font-weight: 600;
    white-space: nowrap;
  }
  .badge.safe {
    background: var(--safe-bg);
    color: var(--safe);
  }
  .badge.recommended {
    background: var(--rec-bg);
    color: var(--rec);
  }
  .badge.critical {
    background: var(--crit-bg);
    color: var(--crit);
  }
  .state-pill {
    font-size: 0.8rem;
    color: var(--safe);
  }
  .state-pill.off {
    color: var(--muted);
  }
  .state {
    color: var(--muted);
    padding: 16px;
  }
  .state.error {
    color: var(--danger);
  }
  .detail {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .detail h2 {
    margin: 0;
    font-size: 1.2rem;
    overflow-wrap: anywhere;
  }
  .detail p {
    margin: 0;
  }
  dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 6px 12px;
    margin: 0;
    font-size: 0.9rem;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  code {
    font-size: 0.85em;
  }
  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .actions button,
  .topbar button,
  .state button,
  .tabs button {
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
  .link {
    background: none;
    border: 0;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
    padding: 0;
  }
  .audit summary {
    cursor: pointer;
    color: var(--muted);
  }
  .audit ul {
    padding-left: 18px;
    margin: 6px 0 0;
  }
  .error-text {
    color: var(--danger);
    margin-left: 6px;
  }
</style>
