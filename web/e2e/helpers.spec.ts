import { expect, test } from '@playwright/test';
import { expectPhoneFit, tabTo } from './helpers';

// Without it, a mobile viewport lays the page out 980 px wide.
const VIEWPORT = '<meta name="viewport" content="width=device-width, initial-scale=1">';

// Self-tests for the helpers on static pages, so a broken check can't pass every screen.
test.describe('helpers', () => {
  test.beforeEach(() => {
    test.skip(test.info().project.name !== 'phone', 'phone width only');
  });

  test('expectPhoneFit passes a page that fits', async ({ page }) => {
    await page.setContent(VIEWPORT + '<button style="width:44px;height:44px">ok</button><input hidden>');
    await expectPhoneFit(page);
  });

  test('expectPhoneFit names a small target', async ({ page }) => {
    await page.setContent(VIEWPORT + '<a href="#" style="display:inline-block;height:20px">tiny</a>');
    await expect(expectPhoneFit(page)).rejects.toThrow(/<a> "tiny" is \d+×20 px/);
  });

  test('expectPhoneFit names the element that scrolls sideways', async ({ page }) => {
    await page.setContent(VIEWPORT + '<div><pre class="wide" style="width:600px">wide</pre></div>');
    await expect(expectPhoneFit(page)).rejects.toThrow(/horizontal scroll.*overflowing <pre\.wide> "wide"/);
  });

  test('tabTo focuses the named element', async ({ page }) => {
    await page.setContent(VIEWPORT + '<button>one</button><button aria-label="two">2</button>');
    await tabTo(page, 'two');
    await expect(page.getByRole('button', { name: 'two' })).toBeFocused();
  });

  test('tabTo fails when nothing has the name', async ({ page }) => {
    await page.setContent(VIEWPORT + '<button>one</button>');
    await expect(tabTo(page, 'missing')).rejects.toThrow('nothing named "missing" got focus after 50 Tab presses');
  });
});
