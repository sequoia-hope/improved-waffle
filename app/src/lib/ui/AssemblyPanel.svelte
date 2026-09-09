<script>
	/**
	 * Left panel for an open Assembly tab (v4 Phase 3c): instances of this
	 * document's Part tabs, mate connectors on instance faces (or explicit
	 * frames), Fastened mates, and the evaluation's problems. Edits go through
	 * the store, which re-evaluates the assembly in the engine
	 * (`OpenAssembly`) and autosaves the tab.
	 */
	import {
		getAssembly,
		getAssemblyStatus,
		getDocumentTabs,
		addInstance,
		updateInstance,
		removeInstance,
		addConnector,
		removeConnector,
		addMate,
		updateMate,
		removeMate,
		getSelectedInstanceId,
		getSelectedInstancePath,
		getSelectedRefs,
		getAssemblyConnectorFrames,
		getConnectorRefusal,
		getSources,
		getSourceTabs,
		getActiveTabId,
		openPartInContext,
		MATE_KINDS
	} from '$lib/engine/store.svelte.js';
	import { eulerDegToQuat, quatToEulerDeg } from '$lib/engine/rotation.js';

	let asm = $derived(getAssembly());
	let status = $derived(getAssemblyStatus());
	let partTabs = $derived(getDocumentTabs().filter((t) => t.kind?.type === 'Part'));
	let assemblyTabs = $derived(getDocumentTabs().filter((t) => t.kind?.type === 'Assembly' && t.id !== getActiveTabId()));
	/** Instance sources: this document's parts and assemblies, then linked documents' tabs. */
	let sourceOptions = $derived.by(() => {
		const opts = [];
		for (const t of partTabs) opts.push({ key: `tab:${t.id}`, tabId: t.id, sourceId: null, label: t.name, group: 'Parts' });
		for (const t of assemblyTabs) opts.push({ key: `tab:${t.id}`, tabId: t.id, sourceId: null, label: `${t.name} (assembly)`, group: 'Assemblies' });
		const tabsBySource = getSourceTabs();
		for (const s of getSources()) {
			for (const t of tabsBySource[s.id] ?? []) {
				if (t.kind !== 'Part' && t.kind !== 'Assembly') continue;
				opts.push({ key: `src:${s.id}:${t.id}`, tabId: t.id, sourceId: s.id, label: `${s.name} › ${t.name}${t.kind === 'Assembly' ? ' (assembly)' : ''}`, group: 'Linked' });
			}
		}
		return opts;
	});
	let selectedInstance = $derived(getSelectedInstanceId());
	let selectedPath = $derived(getSelectedInstancePath());
	/** A same-document Part instance can be edited in this assembly's context. */
	function editableInContext(inst) {
		return !inst.source?.source_id && partTabs.some((t) => t.id === inst.source?.tab_id);
	}
	/**
	 * The pick a connector would be placed on: a face or an edge of the
	 * clicked instance. An edge is what a Revolute mate usually wants (a
	 * hole's rim), and a cylindrical face is what it wants when the rim is
	 * hidden — the engine derives the axis from either.
	 */
	let selectedPick = $derived(
		getSelectedRefs().find((r) => r?.kind?.type === 'Face' || r?.kind?.type === 'Edge') ?? null
	);
	let pickLabel = $derived(selectedPick?.kind?.type === 'Edge' ? 'edge' : 'face');
	/** What each connector's frame was derived from, by connector id. */
	let derivedKinds = $derived.by(() => {
		const by = {};
		for (const f of getAssemblyConnectorFrames()) if (f.kind) by[f.id] = f.kind;
		return by;
	});
	let refusal = $derived(getConnectorRefusal());

	let newInstanceKey = $state('');
	let mateA = $state('');
	let mateB = $state('');
	let mateKind = $state('Fastened');
	let mateFlip = $state(true);
	let mateRotation = $state(0);
	let busy = $state(false);

	function partName(inst) {
		if (inst.source?.source_id) {
			const s = getSources().find((x) => x.id === inst.source.source_id);
			const t = (getSourceTabs()[inst.source.source_id] ?? []).find((x) => x.id === inst.source.tab_id);
			return `${s?.name ?? 'linked'} › ${t?.name ?? inst.source.tab_id}`;
		}
		const tab = getDocumentTabs().find((t) => t.id === inst.source?.tab_id);
		if (!tab) return inst.source?.tab_id ?? '?';
		return tab.kind?.type === 'Assembly' ? `${tab.name} (assembly)` : tab.name;
	}

	function pathLabel(path) {
		if (!path?.length) return '?';
		const top = instanceName(path[0]);
		return path.length > 1 ? `${top} › member` : top;
	}

	function instanceName(id) {
		return asm?.instances.find((i) => i.id === id)?.name ?? '?';
	}

	function connectorName(id) {
		return asm?.connectors?.find((c) => c.id === id)?.name ?? '?';
	}

	const MM = 1000;
	function mm(v) {
		return Math.round((v ?? 0) * MM * 1000) / 1000;
	}

	async function run(fn) {
		busy = true;
		try {
			await fn();
		} finally {
			busy = false;
		}
	}

	async function handleAddInstance() {
		const opt = sourceOptions.find((o) => o.key === newInstanceKey) ?? sourceOptions[0];
		if (!opt) return;
		await run(() => addInstance({ tabId: opt.tabId, sourceId: opt.sourceId }));
	}

	function eulerOf(inst) {
		return quatToEulerDeg(inst.transform?.rotation_quat ?? [0, 0, 0, 1]);
	}

	async function setRotation(inst, axis, valueDeg) {
		const t = JSON.parse(JSON.stringify(inst.transform ?? { translation_m: [0, 0, 0], rotation_quat: [0, 0, 0, 1] }));
		const v = Number(valueDeg);
		if (!Number.isFinite(v)) return;
		const e = eulerOf(inst);
		e[axis] = v;
		t.rotation_quat = eulerDegToQuat(e);
		await run(() => updateInstance(inst.id, { transform: t }));
	}

	async function setTranslation(inst, axis, valueMm) {
		const t = JSON.parse(JSON.stringify(inst.transform ?? { translation_m: [0, 0, 0], rotation_quat: [0, 0, 0, 1] }));
		const v = Number(valueMm);
		if (!Number.isFinite(v)) return;
		t.translation_m[axis] = v / MM;
		await run(() => updateInstance(inst.id, { transform: t }));
	}

	async function handleAddConnectorFromPick() {
		if (!selectedPath?.length || !selectedPick) return;
		await run(() => addConnector({ instancePath: selectedPath, geomRef: selectedPick }));
	}

	async function handleAddOriginConnector(inst) {
		await run(() => addConnector({ instanceId: inst.id, name: `${inst.name} origin` }));
	}

	async function handleAddMate() {
		if (!mateA || !mateB || mateA === mateB) return;
		await run(() => addMate({ a: mateA, b: mateB, kind: mateKind, flip: mateFlip, rotationDeg: Number(mateRotation) || 0 }));
		mateA = '';
		mateB = '';
	}
</script>

{#if asm}
	<div class="assembly-panel" data-testid="assembly-panel">
		<div class="section">
			<div class="section-header">Instances ({asm.instances.length})</div>
			{#each asm.instances as inst, i (inst.id)}
				<div class="row instance" class:selected={selectedInstance === inst.id} data-testid="asm-instance-{i}">
					<div class="row-main">
						<input
							class="name"
							value={inst.name}
							data-testid="asm-instance-name-{i}"
							onchange={(e) => run(() => updateInstance(inst.id, { name: e.currentTarget.value }))}
						/>
						<span class="meta" data-testid="asm-instance-part-{i}">{partName(inst)}</span>
						{#if editableInContext(inst)}
							<button class="act" title="Edit this part in the context of the assembly (the other instances show as ghosts)" data-testid="asm-instance-edit-context-{i}" disabled={busy} onclick={() => run(() => openPartInContext([inst.id]))}>edit</button>
						{/if}
						<button class="act" title="Remove instance" data-testid="asm-instance-remove-{i}" disabled={busy} onclick={() => run(() => removeInstance(inst.id))}>×</button>
					</div>
					<div class="row-sub">
						<label title="Grounded: never moved by mates"><input type="checkbox" data-testid="asm-instance-fixed-{i}" checked={!!inst.fixed} disabled={busy} onchange={(e) => run(() => updateInstance(inst.id, { fixed: e.currentTarget.checked }))} /> fixed</label>
						<label><input type="checkbox" data-testid="asm-instance-suppressed-{i}" checked={!!inst.suppressed} disabled={busy} onchange={(e) => run(() => updateInstance(inst.id, { suppressed: e.currentTarget.checked }))} /> hide</label>
						<span class="xyz">
							{#each ['x', 'y', 'z'] as axis, k}
								<input class="num" type="number" step="0.1" title="{axis} (mm)" data-testid="asm-instance-t{axis}-{i}" value={mm(inst.transform?.translation_m?.[k])} disabled={busy} onchange={(e) => setTranslation(inst, k, e.currentTarget.value)} />
							{/each}
							<span class="unit">mm</span>
						</span>
						<span class="xyz" title="rotation, XYZ Euler (°)">
							{#each ['x', 'y', 'z'] as axis, k}
								<input class="num" type="number" step="5" title="rotate about {axis} (°)" data-testid="asm-instance-r{axis}-{i}" value={eulerOf(inst)[k]} disabled={busy} onchange={(e) => setRotation(inst, k, e.currentTarget.value)} />
							{/each}
							<span class="unit">°</span>
						</span>
						<button class="act" title="Connector at this instance's origin" data-testid="asm-instance-origin-connector-{i}" disabled={busy} onclick={() => handleAddOriginConnector(inst)}>+ frame</button>
					</div>
				</div>
			{/each}
			{#if selectedPath && selectedPath.length > 1}
				<div class="row">
					<span class="meta">selected: {pathLabel(selectedPath)}</span>
					<button class="act" title="Edit the selected member's part in the context of this assembly" data-testid="asm-edit-selected-context" disabled={busy} onclick={() => run(() => openPartInContext(selectedPath))}>edit in context</button>
				</div>
			{/if}
			<div class="row add">
				<select data-testid="asm-add-instance-part" bind:value={newInstanceKey} disabled={sourceOptions.length === 0}>
					{#each ['Parts', 'Assemblies', 'Linked'] as group}
						{#if sourceOptions.some((o) => o.group === group)}
							<optgroup label={group}>
								{#each sourceOptions.filter((o) => o.group === group) as o}
									<option value={o.key}>{o.label}</option>
								{/each}
							</optgroup>
						{/if}
					{/each}
				</select>
				<button class="act primary" data-testid="asm-add-instance" disabled={busy || sourceOptions.length === 0} onclick={handleAddInstance}>+ instance</button>
			</div>
		</div>

		<div class="section">
			<div class="section-header">Mate connectors ({asm.connectors?.length ?? 0})</div>
			{#each asm.connectors ?? [] as c, i (c.id)}
				<div class="row" data-testid="asm-connector-{i}">
					<span class="name-static">{c.name}</span>
					<span class="meta" data-testid="asm-connector-kind-{i}"
						>{pathLabel(c.instance_path)} · {derivedKinds[c.id] ??
							(c.geom_ref ? 'unresolved' : 'explicit frame')}</span
					>
					<button class="act" title="Remove connector" data-testid="asm-connector-remove-{i}" disabled={busy} onclick={() => run(() => removeConnector(c.id))}>×</button>
				</div>
			{/each}
			<div class="row add">
				<button
					class="act primary"
					data-testid="asm-add-connector-face"
					title={selectedInstance && selectedPick
						? `Connector on the selected ${pickLabel} (a planar face, a cylindrical/conical/spherical face, or a circular or straight edge)`
						: 'Select a face or an edge of an instance in the viewport first'}
					disabled={busy || !selectedInstance || !selectedPick}
					onclick={handleAddConnectorFromPick}
				>+ connector on selected {selectedPick ? pickLabel : 'face/edge'}</button>
			</div>
			{#if refusal}
				<div class="row"><span class="err" data-testid="asm-connector-refusal">{refusal}</span></div>
			{/if}
		</div>

		<div class="section">
			<div class="section-header">Mates ({asm.mates?.length ?? 0})</div>
			{#each asm.mates ?? [] as m, i (m.id)}
				<div class="row mate" data-testid="asm-mate-{i}">
					<div class="row-main">
						<span class="name-static">{m.name}</span>
						<span class="meta">{m.kind?.type ?? '?'} · {connectorName(m.connectors?.[0])} → {connectorName(m.connectors?.[1])}</span>
						<button class="act" title="Remove mate" data-testid="asm-mate-remove-{i}" disabled={busy} onclick={() => run(() => removeMate(m.id))}>×</button>
					</div>
					{#if MATE_KINDS.includes(m.kind?.type)}
						<div class="row-sub">
							<select data-testid="asm-mate-kind-{i}" value={m.kind.type} disabled={busy} onchange={(e) => run(() => updateMate(m.id, { kind: e.currentTarget.value }))}>
								{#each MATE_KINDS as k}<option value={k}>{k}</option>{/each}
							</select>
							{#if m.kind.type !== 'Ball'}
								<label><input type="checkbox" data-testid="asm-mate-flip-{i}" checked={!!m.kind.flip} disabled={busy} onchange={(e) => run(() => updateMate(m.id, { flip: e.currentTarget.checked }))} /> flip</label>
							{/if}
							{#if m.kind.type === 'Fastened'}
								<label>rotate <input class="num" type="number" step="15" data-testid="asm-mate-rotation-{i}" value={m.kind.rotation_deg ?? 0} disabled={busy} onchange={(e) => run(() => updateMate(m.id, { rotationDeg: e.currentTarget.value }))} />°</label>
							{/if}
							<label><input type="checkbox" data-testid="asm-mate-suppressed-{i}" checked={!!m.suppressed} disabled={busy} onchange={(e) => run(() => updateMate(m.id, { suppressed: e.currentTarget.checked }))} /> off</label>
						</div>
					{/if}
				</div>
			{/each}
			{#if (asm.connectors?.length ?? 0) >= 2}
				<div class="row add mate-add">
					<select data-testid="asm-mate-a" bind:value={mateA}>
						<option value="">connector A</option>
						{#each asm.connectors as c}<option value={c.id}>{c.name}</option>{/each}
					</select>
					<select data-testid="asm-mate-b" bind:value={mateB}>
						<option value="">connector B</option>
						{#each asm.connectors as c}<option value={c.id}>{c.name}</option>{/each}
					</select>
					<select data-testid="asm-mate-new-kind" bind:value={mateKind} title="Mate kind">
						{#each MATE_KINDS as k}<option value={k}>{k}</option>{/each}
					</select>
					<label><input type="checkbox" data-testid="asm-mate-new-flip" bind:checked={mateFlip} /> flip</label>
					<input class="num" type="number" step="15" data-testid="asm-mate-new-rotation" bind:value={mateRotation} title="rotation about z (°, Fastened)" />
					<button class="act primary" data-testid="asm-add-mate" disabled={busy || !mateA || !mateB || mateA === mateB} onclick={handleAddMate}>mate</button>
				</div>
			{/if}
		</div>

		{#if status?.errors?.length || status?.warnings?.length}
			<div class="section status">
				{#each status.errors ?? [] as e}
					<div class="err" data-testid="asm-error">{e}</div>
				{/each}
				{#each status.warnings ?? [] as w}
					<div class="warn" data-testid="asm-warning">{w}</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}

<style>
	.assembly-panel {
		font-size: 12px;
		color: var(--text-primary, #cdd6f4);
	}
	.section {
		border-bottom: 1px solid var(--border-color, #444);
		padding: 4px 0;
	}
	.section-header {
		padding: 4px 12px;
		font-weight: 600;
		color: var(--text-secondary, #a6adc8);
	}
	.row {
		padding: 3px 12px;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.row.selected {
		background: rgba(0, 120, 212, 0.15);
	}
	.row-main,
	.row-sub,
	.row.add {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
	}
	.row-sub {
		color: var(--text-secondary, #a6adc8);
		font-size: 11px;
	}
	.name {
		flex: 1 1 60px;
		min-width: 0;
		background: transparent;
		border: 1px solid transparent;
		color: inherit;
		padding: 1px 4px;
		border-radius: 3px;
	}
	.name:hover,
	.name:focus {
		border-color: var(--border-color, #45475a);
	}
	.name-static {
		flex: 1 1 60px;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.meta {
		color: var(--text-secondary, #a6adc8);
		font-size: 11px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.xyz {
		display: inline-flex;
		gap: 2px;
		align-items: center;
	}
	.num {
		width: 52px;
		background: var(--bg-primary, #1e1e2e);
		border: 1px solid var(--border-color, #45475a);
		color: inherit;
		border-radius: 3px;
		padding: 1px 3px;
		font-size: 11px;
	}
	.unit {
		font-size: 10px;
	}
	.act {
		padding: 0 6px;
		border-radius: 3px;
		border: 1px solid var(--border-color, #45475a);
		background: transparent;
		color: var(--accent, #89b4fa);
		font-size: 11px;
		cursor: pointer;
	}
	.act.primary {
		border-color: var(--accent, #89b4fa);
	}
	.act:disabled {
		opacity: 0.5;
		cursor: default;
	}
	select {
		background: var(--bg-primary, #1e1e2e);
		color: inherit;
		border: 1px solid var(--border-color, #45475a);
		border-radius: 3px;
		font-size: 11px;
		max-width: 120px;
	}
	.status {
		padding: 4px 12px;
	}
	.err {
		color: var(--error, #f38ba8);
	}
	.warn {
		color: var(--warning, #f9e2af);
	}
</style>
