<script lang="ts">
  // Static content only.
</script>

<section class="glossary" aria-labelledby="glossary-title">
  <h2 id="glossary-title">What the labels mean</h2>
  <dl>
    <dt><span class="badge safe">Safe</span></dt>
    <dd>Well-known software that is generally fine to turn off at login.</dd>
    <dt><span class="badge recommended">Recommended</span></dt>
    <dd>Not in our curated list. Check where it comes from before changing it.</dd>
    <dt><span class="badge critical">Critical</span></dt>
    <dd>Core or suspicious. Turning it off can break things or hide a problem, so you are asked to confirm first.</dd>
    <dt>Boot impact</dt>
    <dd>How much the item slows startup. “Unknown” means the system does not measure it.</dd>
    <dt>Disable vs. delete</dt>
    <dd>Disable keeps the entry so you can turn it back on. Delete removes it permanently.</dd>
  </dl>
</section>

<style>
  .glossary h2 {
    font-size: 1rem;
    margin: 0 0 8px;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 12px;
    margin: 0;
    font-size: 0.9rem;
  }
  dt {
    font-weight: 600;
  }
  dd {
    margin: 0;
    color: var(--muted);
  }
  .badge {
    display: inline-block;
    padding: 1px 8px;
    border-radius: 999px;
    font-size: 0.8rem;
    font-weight: 600;
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
</style>
