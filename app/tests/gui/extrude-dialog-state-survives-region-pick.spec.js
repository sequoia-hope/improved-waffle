/**
 * The extrude dialog's fields survive a region-list change.
 *
 * Region picks, removals, sketch changes and target picks REPLACE the
 * store's dialog-state object (immutable updates). The dialog used to
 * re-seed its local fields from that object on every replacement, so a
 * combine changed to Cut snapped back to Add (or, when editing an existing
 * cut, an Add snapped back to the persisted Cut) and a typed depth reverted
 * the moment the user clicked a region. Seeding is keyed on the dialog
 * SESSION now (open / re-open for a feature), not on object identity.
 */
import { test, expect } from './helpers/waffle-test.js';
import { clickSketch, clickRectangle, clickFinishSketch, clickExtrude } from './helpers/toolbar.js';
import { drawRectangle } from './helpers/canvas.js';
import { waitForEntityCount, waitForFeatureCount, waitForMeshWithGeometry } from './helpers/state.js';

async function sketchRectangle(waffle) {
	await clickSketch(waffle.page);
	await clickRectangle(waffle.page);
	await drawRectangle(waffle.page, -80, -60, 80, 60);
	await waitForEntityCount(waffle.page, 8, 5000);
	await clickFinishSketch(waffle.page);
	await waitForFeatureCount(waffle.page, 1, 10000);
}

/** The first sketch feature's `{ id, name }`. */
async function firstSketch(page) {
	return page.evaluate(() => {
		const s = window.__waffle.getFeatureTree().features.find((f) => f.operation?.type === 'Sketch');
		return { id: s.id, name: s.name };
	});
}

test.describe('extrude dialog state survives region changes', () => {
	test('combine and depth survive removing and re-adding a region', async ({ waffle }) => {
		await sketchRectangle(waffle);
		await clickExtrude(waffle.page);

		const combine = waffle.page.locator('[data-testid="extrude-combine"]');
		const depth = waffle.page.locator('[data-testid="extrude-depth"]');
		await depth.fill('42');
		await combine.selectOption('Cut');
		await expect(combine).toHaveValue('Cut');

		// Removing the auto-added region replaces the dialog-state object.
		await waffle.page.locator('[data-testid="extrude-region-0"] .region-remove').click();
		await expect(waffle.page.locator('[data-testid="extrude-region-0"]')).not.toBeVisible();
		await expect(combine).toHaveValue('Cut');
		await expect(depth).toHaveValue('42');

		// So does adding one back (the viewport region click's store path).
		const sketch = await firstSketch(waffle.page);
		await waffle.page.evaluate(
			({ id, name }) => window.__waffle.addExtrudeRegion(id, name, 0),
			sketch
		);
		await expect(waffle.page.locator('[data-testid="extrude-region-0"]')).toBeVisible();
		await expect(combine).toHaveValue('Cut');
		await expect(depth).toHaveValue('42');

		// A fresh session still starts from defaults.
		await waffle.page.locator('[data-testid="extrude-cancel"]').click();
		await clickExtrude(waffle.page);
		await expect(combine).toHaveValue('Add');
		await expect(depth).toHaveValue('10');
	});

	test('editing a Cut extrude: a change away from Cut survives a region change', async ({ waffle }) => {
		await sketchRectangle(waffle);
		await clickExtrude(waffle.page);
		await waffle.page.locator('[data-testid="extrude-depth"]').fill('10');
		await waffle.page.locator('[data-testid="extrude-apply"]').click();
		await waitForFeatureCount(waffle.page, 2, 15000);
		await waitForMeshWithGeometry(waffle.page);

		// Re-open the extrude for editing and switch it to a standalone body.
		const extrudeId = await waffle.page.evaluate(
			() => window.__waffle.getFeatureTree().features.find((f) => f.operation?.type === 'Extrude').id
		);
		await waffle.page.evaluate((id) => window.__waffle.showEditFeatureDialog(id), extrudeId);
		const combine = waffle.page.locator('[data-testid="extrude-combine"]');
		await expect(combine).toHaveValue('Add');
		await combine.selectOption('NewBody');

		// A region-list change must not restore the persisted mode.
		await waffle.page.locator('[data-testid="extrude-region-0"] .region-remove').click();
		await expect(combine).toHaveValue('NewBody');
		const sketch = await firstSketch(waffle.page);
		await waffle.page.evaluate(
			({ id, name }) => window.__waffle.addExtrudeRegion(id, name, 0),
			sketch
		);
		await expect(combine).toHaveValue('NewBody');
		await waffle.page.locator('[data-testid="extrude-cancel"]').click();
	});
});
