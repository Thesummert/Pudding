import { test, expect } from '@playwright/test';

test('background images and adjustments persist and can be removed', async ({ page }) => {
  await page.goto(process.env.PUDDING_E2E_FILE || '/');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: '设置' });
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/aWQAAAAASUVORK5CYII=', 'base64');
  await dialog.locator('input[type=file]').setInputFiles([
    { name: 'first.png', mimeType: 'image/png', buffer: png },
    { name: 'second.png', mimeType: 'image/png', buffer: png },
  ]);
  await expect(dialog.getByRole('button', { name: '使用背景 second.png' })).toBeVisible();
  await dialog.getByRole('button', { name: '使用背景 second.png' }).click();
  await dialog.getByRole('button', { name: '随机选择' }).click();
  await expect(dialog.getByRole('button', { name: '使用背景 first.png' })).toHaveAttribute('aria-pressed', 'true');
  await dialog.getByRole('button', { name: '随机选择' }).click();
  await expect(dialog.getByRole('button', { name: '使用背景 second.png' })).toHaveAttribute('aria-pressed', 'true');
  await dialog.getByLabel('背景高斯模糊').fill('12');
  await dialog.getByLabel('背景不透明度').fill('45');
  await dialog.getByLabel('启动时随机切换概率').fill('0');
  await expect(page.locator('.window-background img')).toHaveCSS('filter', 'blur(12px)');
  const bounds = await page.locator('.window-background').boundingBox();
  expect(bounds).toMatchObject({ x: 0, y: 0, width: 1280, height: 820 });
  await expect(page.locator('.sidebar')).not.toHaveCSS('background-color', 'rgb(248, 249, 252)');
  await page.reload();
  await expect(page.locator('.window-background img')).toHaveCSS('opacity', '0.45');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  await expect(dialog.getByRole('button', { name: '使用背景 second.png' })).toHaveAttribute('aria-pressed', 'true');
  await dialog.getByLabel('启动时随机切换概率').fill('100');
  await page.reload();
  await page.getByRole('button', { name: '设置', exact: true }).click();
  await expect(dialog.getByRole('button', { name: '使用背景 first.png' })).toHaveAttribute('aria-pressed', 'true');
  await dialog.getByRole('button', { name: '使用背景 second.png' }).click();
  await dialog.getByRole('button', { name: '移除背景 second.png' }).click();
  await expect(page.locator('.window-background')).toHaveCount(0);
  await expect(dialog.getByRole('button', { name: '使用背景 first.png' })).toBeVisible();
});

test('background selection stays in sync between open windows', async ({ page, context }) => {
  await page.goto(process.env.PUDDING_E2E_FILE || '/');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: '设置' });
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/aWQAAAAASUVORK5CYII=', 'base64');
  await dialog.locator('input[type=file]').setInputFiles([
    { name: 'first.png', mimeType: 'image/png', buffer: png },
    { name: 'second.png', mimeType: 'image/png', buffer: png },
  ]);
  await expect(dialog.getByRole('button', { name: '使用背景 second.png' })).toBeVisible();
  await dialog.getByLabel('启动时随机切换概率').fill('0');
  const other = await context.newPage();
  await other.goto(process.env.PUDDING_E2E_FILE || '/');
  await other.getByRole('button', { name: '设置', exact: true }).click();
  const otherDialog = other.getByRole('dialog', { name: '设置' });
  await expect(otherDialog.getByRole('button', { name: '使用背景 first.png' })).toHaveAttribute('aria-pressed', 'true');
  await dialog.getByRole('button', { name: '使用背景 second.png' }).click();
  await expect(otherDialog.getByRole('button', { name: '使用背景 second.png' })).toHaveAttribute('aria-pressed', 'true');
  await otherDialog.getByLabel('背景不透明度').fill('60');
  await page.reload();
  await page.getByRole('button', { name: '设置', exact: true }).click();
  await expect(dialog.getByRole('button', { name: '使用背景 second.png' })).toHaveAttribute('aria-pressed', 'true');
  await expect(dialog.getByLabel('背景不透明度')).toHaveValue('60');
});
