import { test, expect } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.goto(process.env.PUDDING_E2E_FILE || '/');
  await expect(page.getByRole('heading', { name: '主文件夹.' })).toBeVisible();
});

test('directory history, parent navigation and typed paths', async ({ page }) => {
  await page.getByRole('button', { name: 'Documents', exact: true }).dblclick();
  await expect(page.getByRole('heading', { name: '文档.' })).toBeVisible();
  await page.getByRole('button', { name: '后退', exact: true }).click();
  await expect(page.getByRole('heading', { name: '主文件夹.' })).toBeVisible();
  await page.getByRole('button', { name: '前进', exact: true }).click();
  await expect(page.getByRole('heading', { name: '文档.' })).toBeVisible();
  await page.getByRole('button', { name: '上一级', exact: true }).click();
  await page.getByRole('button', { name: '编辑路径', exact: true }).click();
  await page.getByRole('textbox', { name: '文件夹路径' }).fill('~/Downloads');
  await page.getByRole('textbox', { name: '文件夹路径' }).press('Enter');
  await expect(page.getByRole('button', { name: '设计素材.zip', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '编辑路径', exact: true }).click();
  await page.getByRole('textbox', { name: '文件夹路径' }).fill('/does-not-exist');
  await page.getByRole('textbox', { name: '文件夹路径' }).press('Enter');
  await expect(page.getByRole('alert')).toContainText('演示目录不存在');
  await expect(page.getByRole('heading', { name: '下载.' })).toBeVisible();
});

test('filtering, hidden files, view preferences and theme', async ({ page }) => {
  await expect(page.getByRole('button', { name: '.config', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: '更多显示选项', exact: true }).click();
  await page.getByRole('menuitemcheckbox', { name: '显示隐藏文件' }).click();
  await expect(page.getByRole('button', { name: '.config', exact: true })).toBeVisible();
  await page.getByRole('textbox', { name: '搜索当前文件夹' }).fill('document');
  await expect(page.locator('.file-card')).toHaveCount(1);
  await page.getByRole('button', { name: '清除搜索', exact: true }).click();
  await page.getByRole('button', { name: '列表视图', exact: true }).click();
  await expect(page.locator('.file-row')).toHaveCount(11);
  await page.getByRole('button', { name: '切换深色外观', exact: true }).click();
  await page.reload();
  await expect(page.locator('.app-shell')).toHaveAttribute('data-theme', 'dark');
  await expect(page.locator('.file-row')).toHaveCount(11);
  await page.getByRole('textbox', { name: '搜索当前文件夹' }).fill('unmatched-file');
  await expect(page.getByRole('heading', { name: '没有找到匹配的项目' })).toBeVisible();
});

test('create, rename, conflict validation and trash a folder', async ({ page }) => {
  await page.getByRole('button', { name: '新建文件夹', exact: true }).click();
  await page.getByRole('textbox', { name: '名称', exact: true }).fill('测试文件夹');
  await page.getByRole('button', { name: '创建文件夹', exact: true }).click();
  const item = page.getByRole('button', { name: '测试文件夹', exact: true });
  await expect(item).toBeVisible();
  await item.click({ button: 'right' });
  await page.getByRole('menuitem', { name: '重命名' }).click();
  await page.getByRole('textbox', { name: '名称', exact: true }).fill('Documents');
  await page.getByRole('button', { name: '保存名称', exact: true }).click();
  await expect(page.locator('.field-error')).toContainText('同名');
  await page.getByRole('textbox', { name: '名称', exact: true }).fill('已重命名');
  await page.getByRole('button', { name: '保存名称', exact: true }).click();
  const renamed = page.getByRole('button', { name: '已重命名', exact: true });
  await expect(renamed).toBeVisible();
  await renamed.click({ button: 'right' });
  await page.getByRole('menuitem', { name: '移入回收站' }).click();
  await page.getByRole('dialog').getByRole('button', { name: '移入回收站', exact: true }).click();
  await expect(renamed).toHaveCount(0);
});

test('copy and cut between folders without overwriting existing files', async ({ page }) => {
  await page.getByRole('button', { name: '开始使用.md', exact: true }).click({ button: 'right' });
  await page.locator('.context-menu').getByRole('menuitem').filter({ hasText: /^复制/ }).first().click();
  await page.getByRole('navigation', { name: '常用位置' }).getByRole('button', { name: '文档', exact: true }).click();
  await page.getByRole('button', { name: '粘贴到此处', exact: false }).click();
  const copy = page.getByRole('button', { name: '开始使用.md', exact: true });
  await expect(copy).toBeVisible();
  await page.getByRole('button', { name: '粘贴到此处', exact: false }).click();
  await expect(page.getByRole('alert')).toContainText('同名');
  await copy.click({ button: 'right' });
  await page.getByRole('menuitem', { name: /剪切/ }).click();
  await page.getByRole('navigation', { name: '常用位置' }).getByRole('button', { name: '下载', exact: true }).click();
  await page.getByRole('button', { name: '粘贴到此处', exact: false }).click();
  await expect(page.getByRole('button', { name: '开始使用.md', exact: true })).toBeVisible();
  await page.getByRole('navigation', { name: '常用位置' }).getByRole('button', { name: '文档', exact: true }).click();
  await expect(page.getByRole('button', { name: '开始使用.md', exact: true })).toHaveCount(0);
});

test('selection, keyboard navigation and preview disclosure', async ({ page }) => {
  await page.getByRole('button', { name: '开始使用.md', exact: true }).click();
  await expect(page.getByRole('complementary', { name: '信息面板' })).toContainText('2.4 KB');
  await page.keyboard.press('ArrowLeft');
  await expect(page.locator('.file-card[aria-pressed="true"]')).toHaveCount(1);
  await page.getByRole('button', { name: '开始使用.md', exact: true }).dblclick();
  await expect(page.getByRole('alert')).toContainText('浏览器演示无法启动本地应用');
  await page.getByRole('button', { name: '关闭信息面板', exact: true }).click();
  await expect(page.getByRole('complementary', { name: '信息面板' })).toHaveCount(0);
  await expect(page.getByText('演示模式', { exact: true })).toBeVisible();
});
