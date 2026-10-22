<script lang="ts">
  import { onMount } from 'svelte';

  type Asset = {
    id: string;
    name: string;
    details: string;
    value: number;
    status: string;
    statusLabel: string;
    note?: string;
    clientComment?: string;
    valueHelp?: string;
    excludedFromTotal?: boolean;
  };

  type Category = { id: string; name: string; icon: string; assets: Asset[] };
  type AssetsPage = {
    client: { name: string; coClient: string; reportTitle: string };
    notice: string;
    funding: {
      percent: number;
      verified: number;
      leftToVerify: number;
      notFundable: number;
      notFundedPerClient: number;
      fundableTotal: number;
      footnote: string;
    };
    nextSteps: { title: string; description: string }[];
    categories: Category[];
    totalValue: number;
    totalNote: string;
    footer: { firm: string; address: string; phone: string; links: string[] };
  };

  let page = $state<AssetsPage | null>(null);
  let loading = $state(true);
  let loadError = $state('');
  let search = $state('');
  let collapsed = $state<Record<string, boolean>>({});
  let draftValues = $state<Record<string, number>>({});
  let commentDrafts = $state<Record<string, string>>({});
  let openComments = $state<Record<string, boolean>>({});
  let modalOpen = $state(false);
  let newAssetName = $state('');
  let newAssetCategory = $state('');
  let newAssetValue = $state<number | ''>('');
  let toast = $state('');

  onMount(async () => {
    try {
      await loadAssets();
    } catch (error) {
      loadError = error instanceof Error ? error.message : 'Unable to load assets.';
    } finally {
      loading = false;
    }
  });

  async function loadAssets() {
    const response = await fetch('/api/assets');
    if (!response.ok) throw new Error(`Request failed (${response.status})`);
    page = await response.json() as AssetsPage;
    newAssetCategory ||= page.categories[0]?.id ?? '';
  }

  async function mutateAsset(url: string, method: 'POST' | 'PUT' | 'DELETE', body?: unknown) {
    const response = await fetch(url, {
      method,
      headers: body ? { 'Content-Type': 'application/json' } : undefined,
      body: body ? JSON.stringify(body) : undefined
    });
    if (!response.ok) {
      const result = await response.json().catch(() => null) as { error?: string } | null;
      throw new Error(result?.error ?? `Request failed (${response.status})`);
    }
  }

  const visibleCategories = $derived(page?.categories
    .map((category) => ({
      ...category,
      assets: category.assets.filter((asset) =>
        `${asset.name} ${asset.details} ${asset.statusLabel}`.toLowerCase().includes(search.trim().toLowerCase())
      )
    }))
    .filter((category) => category.assets.length > 0 || (!search.trim() && category.assets.length === 0)) ?? []);

  const draftCount = $derived(Object.keys(draftValues).length);

  const totalValue = $derived((page?.totalValue ?? 0) + (page?.categories ?? []).reduce((total, category) =>
    total + category.assets.reduce((categoryTotal, asset) => {
      if (asset.excludedFromTotal || asset.status === 'not_fundable' || asset.status === 'not_funded') return categoryTotal;
      const originalValue = asset.id.startsWith('draft-') ? 0 : asset.value;
      const proposedValue = draftValues[asset.id] ?? asset.value;
      return categoryTotal + proposedValue - originalValue;
    }, 0), 0));

  function money(value: number) {
    return new Intl.NumberFormat('en-US', {
      style: 'currency', currency: 'USD', maximumFractionDigits: 0
    }).format(value);
  }

  function toggleCategory(id: string) {
    collapsed[id] = !collapsed[id];
  }

  function updateValue(asset: Asset, event: Event) {
    const value = Number((event.currentTarget as HTMLInputElement).value);
    if (Number.isFinite(value)) draftValues[asset.id] = value;
  }

  function openComment(assetId: string) {
    openComments[assetId] = true;
    let asset: Asset | undefined;
    for (const category of page?.categories ?? []) {
      asset = category.assets.find((item) => item.id === assetId);
      if (asset) break;
    }
    commentDrafts[assetId] ??= asset?.clientComment ?? '';
  }

  async function saveComment(assetId: string) {
    const clientComment = commentDrafts[assetId]?.trim();
    if (!clientComment) return;
    try {
      await mutateAsset(`/api/assets/${encodeURIComponent(assetId)}`, 'PUT', { clientComment });
      await loadAssets();
      openComments[assetId] = false;
      toast = 'Comment saved to the assets JSON file.';
    } catch (error) {
      toast = error instanceof Error ? error.message : 'Unable to save comment.';
    }
  }

  async function saveValue(asset: Asset) {
    const value = draftValues[asset.id];
    if (value === undefined || value === asset.value) return;
    try {
      await mutateAsset(`/api/assets/${encodeURIComponent(asset.id)}`, 'PUT', { value });
      await loadAssets();
      const { [asset.id]: _savedValue, ...remainingValues } = draftValues;
      draftValues = remainingValues;
      toast = 'Asset value saved to the JSON file.';
    } catch (error) {
      toast = error instanceof Error ? error.message : 'Unable to save asset value.';
    }
  }

  async function deleteAsset(asset: Asset) {
    if (!window.confirm(`Permanently delete “${asset.name}” from the assets JSON file?`)) return;
    try {
      await mutateAsset(`/api/assets/${encodeURIComponent(asset.id)}`, 'DELETE');
      await loadAssets();
      toast = 'Asset deleted from the JSON file.';
    } catch (error) {
      toast = error instanceof Error ? error.message : 'Unable to delete asset.';
    }
  }

  async function addAsset(event: SubmitEvent) {
    event.preventDefault();
    if (!page || !newAssetName.trim() || newAssetValue === '') return;
    try {
      await mutateAsset('/api/assets', 'POST', {
        categoryId: newAssetCategory,
        name: newAssetName.trim(),
        details: 'Added by client',
        value: Number(newAssetValue)
      });
      await loadAssets();
      newAssetName = '';
      newAssetValue = '';
      modalOpen = false;
      toast = 'Asset added to the JSON file.';
    } catch (error) {
      toast = error instanceof Error ? error.message : 'Unable to add asset.';
    }
  }

  async function submitChanges() {
    const pendingValues = Object.entries(draftValues);
    if (!pendingValues.length) {
      toast = 'All changes are already saved to the JSON file.';
      return;
    }
    try {
      await Promise.all(pendingValues.map(([id, value]) =>
        mutateAsset(`/api/assets/${encodeURIComponent(id)}`, 'PUT', { value })
      ));
      await loadAssets();
      draftValues = {};
      toast = 'Asset values saved to the JSON file.';
    } catch (error) {
      toast = error instanceof Error ? error.message : 'Unable to save asset values.';
    }
  }
</script>

<svelte:head>
  <title>My Assets | InSight Law</title>
  <meta name="description" content="Review and update the assets in your estate plan." />
</svelte:head>

{#if loading}
  <div class="state-screen" role="status"><span class="loading-mark"></span><p>Loading your asset report</p></div>
{:else if loadError}
  <div class="state-screen error-screen" role="alert"><strong>We couldn't load your assets.</strong><p>{loadError}. Confirm the Rust API is running on port 3000.</p><button class="button button-primary" onclick={() => location.reload()}>Try again</button></div>
{:else if page}
  <div class="portal">
    <header class="topbar">
      <a class="brand" href="/" aria-label="InSight Law home"><span class="brand-mark">I</span><span><strong>INSIGHT</strong><small>LAW · CLIENT PORTAL</small></span></a>
      <div class="topbar-actions">
        <button class="icon-button message-button" aria-label="Secure messages" title="Secure messages"><span aria-hidden="true">✉</span><span class="message-label">Messages</span></button>
        <span class="topbar-rule"></span>
        <button class="profile-button" aria-label="Client profile"><span class="avatar">W</span><span class="profile-copy"><strong>{page.client.name}</strong><small>Client account</small></span><span class="profile-chevron">⌄</span></button>
      </div>
    </header>

    <aside class="sidebar" aria-label="Main navigation">
      <div class="nav-label">CLIENT PORTAL</div>
      <a class="nav-item" href="#home"><span class="nav-icon">⌂</span>Home</a>
      <a class="nav-item active" href="#assets" aria-current="page"><span class="nav-icon">▤</span>My Assets<span class="active-bar"></span></a>
      <a class="nav-item" href="#contacts"><span class="nav-icon">♧</span>My Contacts</a>
      <a class="nav-item" href="#documents"><span class="nav-icon">▧</span>My Documents</a>
      <a class="nav-item" href="#messages"><span class="nav-icon">✉</span>Messages</a>
      <a class="nav-item" href="#resources"><span class="nav-icon">▤</span>Resources</a>
      <div class="sidebar-bottom"><span class="sidebar-divider"></span><a class="nav-item sign-out" href="#sign-out"><span class="nav-icon">↪</span>Sign Out</a><div class="secure-side"><span class="secure-dot"></span>Secure client portal</div></div>
    </aside>

    <main id="assets" class="main-content">
      <div class="content-wrap">
        <div class="page-kicker">ESTATE PLAN <span>/</span> MY ASSETS</div>
        <div class="trust-reminder"><span class="trust-icon">✓</span><span>{page.notice}</span><span class="reminder-tag">ACTION NEEDED</span></div>

        <section class="overview" aria-label="Funding progress and next steps">
          <div class="funding-panel">
            <div class="panel-heading"><div><span class="eyebrow">YOUR ESTATE PLAN</span><h2>Funding progress</h2></div><button class="info-button" title="Funding progress excludes assets marked not fundable." aria-label="About funding progress">i</button></div>
            <div class="funding-content">
              <div class="gauge-wrap">
                <svg class="gauge" viewBox="0 0 320 178" role="img" aria-label={`${page.funding.percent}% of fundable assets verified`}>
                  <path class="gauge-track" d="M 34 148 A 126 126 0 0 1 286 148" pathLength="100"></path>
                  <path class="gauge-progress" d="M 34 148 A 126 126 0 0 1 286 148" pathLength="100" stroke-dasharray={`${page.funding.percent} 100`}></path>
                  <text class="gauge-value" x="160" y="92" text-anchor="middle">{page.funding.percent}%</text>
                  <text class="gauge-label" x="160" y="118" text-anchor="middle">funded</text>
                  <text class="gauge-caption" x="160" y="148" text-anchor="middle">{page.funding.verified} of {page.funding.fundableTotal} verified</text>
                </svg>
              </div>
              <div class="funding-stats">
                <div class="stat"><span class="stat-dot verified-dot"></span><span>Verified</span><strong>{page.funding.verified}</strong></div>
                <div class="stat"><span class="stat-dot review-dot"></span><span>Left to verify</span><strong>{page.funding.leftToVerify}</strong></div>
                <div class="stat"><span class="stat-dot neutral-dot"></span><span>Not fundable</span><strong>{page.funding.notFundable}</strong></div>
                <div class="stat"><span class="stat-dot attention-dot"></span><span>Not funded per client</span><strong>{page.funding.notFundedPerClient}</strong></div>
              </div>
            </div>
            <p class="panel-footnote"><span>i</span>{page.funding.footnote}</p>
          </div>

          <aside class="next-panel" aria-label="What to do next">
            <div class="panel-heading"><div><span class="eyebrow">YOUR CHECKLIST</span><h2>What to do next</h2></div><span class="step-count">{page.nextSteps.length} STEPS</span></div>
            <ol class="next-list">
              {#each page.nextSteps as step, index}
                <li><span class="step-number">0{index + 1}</span><span class="step-copy"><strong>{step.title}</strong><small>{step.description}</small></span><span class="step-arrow">›</span></li>
              {/each}
            </ol>
            <p class="next-note">When you're done, submit your updates for firm review.</p>
          </aside>
        </section>

        <section class="report-section" aria-label="Asset review report">
          <div class="report-heading">
            <div><span class="eyebrow">CLIENT ASSET INVENTORY</span><h1>{page.client.reportTitle}</h1><p>Review asset details, update values, and keep the report current.</p></div>
            <button class="button button-outline add-button" onclick={() => modalOpen = true}><span aria-hidden="true">+</span> Add asset</button>
          </div>

          {#if draftCount > 0}
            <div class="draft-banner" role="status"><span class="draft-symbol">!</span><span><strong>{draftCount} unsaved value {draftCount === 1 ? 'edit' : 'edits'}</strong> will be written to the JSON file.</span><button class="text-button" onclick={submitChanges}>Save now <span aria-hidden="true">→</span></button></div>
          {/if}

          <div class="toolbar"><label class="search-box"><span aria-hidden="true">⌕</span><input aria-label="Search assets" placeholder="Search assets" bind:value={search} /></label><span class="asset-count">{visibleCategories.reduce((count, category) => count + category.assets.length, 0)} assets shown</span></div>
          <div class="asset-table" role="table" aria-label="Assets list">
            <div class="table-header" role="row"><span role="columnheader">Asset description</span><span role="columnheader">Asset value</span><span role="columnheader">Funding status</span><span role="columnheader">Comments</span><span aria-hidden="true"></span></div>
            {#each visibleCategories as category (category.id)}
              <div class="category-block">
                <button class="category-heading" aria-expanded={!collapsed[category.id]} onclick={() => toggleCategory(category.id)}>
                  <span class:collapsed={collapsed[category.id]} class="category-chevron" aria-hidden="true">⌄</span><span class="category-icon">{category.icon}</span><span class="category-title">{category.name}</span><span class="category-count">{category.assets.length}</span><span class="category-line"></span>
                </button>
                {#if !collapsed[category.id]}
                  {#if category.assets.length === 0}
                    <div class="empty-category">No {category.name.toLowerCase()} listed. Use <button class="inline-link" onclick={() => { newAssetCategory = category.id; modalOpen = true; }}>Add asset</button> to include one.</div>
                  {:else}
                    {#each category.assets as asset (asset.id)}
                      <article class="asset-row">
                        <div class="asset-description"><strong>{asset.name}</strong><span>{asset.details}</span>{#if asset.valueHelp}<small class="asset-help">{asset.valueHelp}</small>{/if}</div>
                        <div class="asset-value"><span class="mobile-label">Asset value</span><label class="value-field"><span class="currency-prefix">$</span><input aria-label={`Value for ${asset.name}`} type="number" min="-999999999" step="1" value={draftValues[asset.id] ?? asset.value} oninput={(event) => updateValue(asset, event)} onblur={() => saveValue(asset)} /></label>{#if asset.excludedFromTotal}<small class="excluded-note">* Excluded from total</small>{/if}</div>
                        <div class="asset-status"><span class="mobile-label">Funding status</span><span class={`status-pill ${asset.status}`}>{#if asset.status === 'verified'}✓{:else if asset.status === 'needs_verification'}!{:else if asset.status === 'undecided'}?{:else}−{/if} {asset.statusLabel}</span></div>
                        <div class="asset-comments"><span class="mobile-label">Comments</span>{#if asset.note}<p class="firm-note">{asset.note}</p>{/if}{#if asset.clientComment}<p class="saved-comment">{asset.clientComment}</p>{/if}{#if !asset.clientComment}<button class="comment-link" onclick={() => openComment(asset.id)}>{asset.note ? 'Reply to firm' : 'Add comment'} <span>(optional)</span></button>{/if}{#if openComments[asset.id]}<div class="comment-editor"><textarea maxlength="500" placeholder="Type your reply or comment..." bind:value={commentDrafts[asset.id]}></textarea><div class="comment-editor-footer"><span>{commentDrafts[asset.id]?.length ?? 0}/500</span><div><button class="text-button cancel-comment" onclick={() => openComments[asset.id] = false}>Cancel</button><button class="mini-primary" onclick={() => saveComment(asset.id)}>Save to JSON</button></div></div></div>{/if}</div>
                        <div class="asset-actions"><button class="remove-button" aria-label={`Delete ${asset.name}`} title="Delete asset" onclick={() => deleteAsset(asset)}>×</button></div>
                      </article>
                    {/each}
                  {/if}
                {/if}
              </div>
            {/each}
            {#if visibleCategories.length === 0}<div class="no-results">No assets match “{search}”.</div>{/if}
          </div>

          <div class="bottom-actions"><div class="total-copy"><span>ESTIMATED TOTAL</span><strong>{money(totalValue)}</strong><small>{page.totalNote}</small></div><div class="bottom-buttons"><button class="button button-outline" onclick={() => modalOpen = true}><span aria-hidden="true">+</span> Add asset</button><button class="button button-primary submit-button" onclick={submitChanges} disabled={draftCount === 0}>Save values <span aria-hidden="true">→</span></button></div></div>
        </section>
      </div>

      <footer class="page-footer"><div><strong>{page.footer.firm}</strong><span>{page.footer.address}</span><a href={`tel:${page.footer.phone}`}>{page.footer.phone}</a></div><nav aria-label="Legal links">{#each page.footer.links as link}<a href="#footer">{link}</a>{/each}</nav><span class="footer-secure"><span class="secure-dot"></span>Secure client portal</span></footer>
    </main>
  </div>

  {#if modalOpen}
    <div class="modal-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) modalOpen = false; }}>
      <div class="asset-modal" role="dialog" aria-modal="true" aria-labelledby="modal-title" tabindex="-1">
        <div class="modal-heading"><div><span class="eyebrow">CLIENT ASSET INVENTORY</span><h2 id="modal-title">Add an asset</h2></div><button class="modal-close" aria-label="Close" onclick={() => modalOpen = false}>×</button></div>
        <form onsubmit={addAsset}>
          <label>Asset category<select required bind:value={newAssetCategory}>{#each page.categories as category}<option value={category.id}>{category.name}</option>{/each}</select></label>
          <label>Asset description<input required maxlength="120" bind:value={newAssetName} placeholder="For example, savings account" /></label>
          <label>Estimated value<input required type="number" min="0" step="1" bind:value={newAssetValue} placeholder="0" /></label>
          <p class="modal-note">The new asset will be saved to the page JSON immediately.</p>
          <div class="modal-actions"><button type="button" class="button button-quiet" onclick={() => modalOpen = false}>Cancel</button><button type="submit" class="button button-primary">Add asset</button></div>
        </form>
      </div>
    </div>
  {/if}
  {#if toast}<div class="toast" role="status">{toast}<button aria-label="Dismiss notification" onclick={() => toast = ''}>×</button></div>{/if}
{/if}
