import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import Palette from './Palette.svelte';
import CaptureBar from '../capture/CaptureBar.svelte';
import { CAPTURE_OPEN, COMMANDS, PALETTE_OPEN } from './commands';
import type { Hit } from '../lib/search';
import { makeObj } from '../objects/testing';

type Call = { url: string; signal?: AbortSignal | null };

/** Stub fetch: `/api/search` answers via `reply` (default: no results). */
function stub(reply: (call: Call) => Promise<Response> = async () => Response.json({ results: [], total: 0 })) {
  const calls: Call[] = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string, init?: RequestInit) => {
      const call = { url, signal: init?.signal };
      calls.push(call);
      return url.startsWith('/api/search?') ? reply(call) : new Response(null, { status: 599 });
    }),
  );
  return calls;
}

const page = (...results: Hit[]) => Response.json({ results, total: results.length });

function setup() {
  document.body.innerHTML = '<input aria-label="other" />';
  const other = screen.getByRole('textbox', { name: 'other' });
  other.focus();
  render(Palette);
  return other;
}

async function openPalette(target: Element | Window = document.activeElement ?? window) {
  const key = new KeyboardEvent('keydown', { key: 'k', code: 'KeyK', ctrlKey: true, bubbles: true, cancelable: true });
  target.dispatchEvent(key);
  expect(key.defaultPrevented).toBe(true);
  const input = await screen.findByRole('combobox', { name: 'palette' });
  await waitFor(() => expect(input).toHaveFocus());
  return input as HTMLInputElement;
}

const dialog = () => screen.queryByRole('dialog', { name: 'palette' });
const options = () => screen.queryAllByRole('option');
const selected = () => screen.getByRole('option', { selected: true });

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
  location.hash = '';
});

describe('command palette', () => {
  it('Ctrl+K opens it from another input; Esc closes it and restores focus', async () => {
    stub();
    const other = setup();
    expect(dialog()).toBeNull();
    const input = await openPalette(other);
    expect(options().map((o) => o.textContent?.trim())).toEqual(COMMANDS.map((c) => c.label));
    const esc = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    input.dispatchEvent(esc);
    expect(esc.defaultPrevented).toBe(true);
    await waitFor(() => expect(dialog()).toBeNull());
    expect(other).toHaveFocus();
  });

  it('filters commands as you type, then lists search results with markers', async () => {
    const calls = stub(async () =>
      page(
        { object: makeObj(7, { title: 'Today plan', archived: true }), snippet: '[Today] plan', score: 16 },
        { object: { ...makeObj(8, { type: 'task', title: 'todo list' }), task: { status: 'done' } }, snippet: '[tod]o list', score: 12 },
      ),
    );
    setup();
    const input = await openPalette();
    await fireEvent.input(input, { target: { value: 'TODAY' } });
    await waitFor(() => expect(options()).toHaveLength(4));
    expect(calls.at(-1)?.url).toBe('/api/search?q=TODAY&limit=10');
    const [cmd, other, archived, done] = options();
    expect(cmd).toHaveTextContent('go to today');
    expect(other).toHaveTextContent('go to someday');
    expect(archived).toHaveTextContent('note Today plan [archived]');
    expect(archived).not.toHaveTextContent('[done]');
    expect(done).toHaveTextContent('task todo list [done]');
    expect(archived.querySelector('mark')).toHaveTextContent('Today');
    expect(done.querySelector('.snippet')).toHaveTextContent('todo list');
    expect(selected()).toBe(cmd);
    expect(input).toHaveAttribute('aria-activedescendant', cmd.id);
  });

  it('shows no matches when nothing matches', async () => {
    stub();
    setup();
    const input = await openPalette();
    await fireEvent.input(input, { target: { value: 'zzz' } });
    expect(options()).toEqual([]);
    expect(dialog()).toHaveTextContent('no matches');
  });

  it('shows a search failure inline and keeps the commands', async () => {
    stub(async () => Response.json({ error: 'boom' }, { status: 500 }));
    setup();
    const input = await openPalette();
    await fireEvent.input(input, { target: { value: 'new' } });
    expect(await screen.findByRole('alert')).toHaveTextContent('search: boom');
    expect(options().map((o) => o.textContent?.trim())).toEqual(['new task']);
  });

  it('aborts the previous search and ignores a stale response', async () => {
    const replies: Array<(r: Response) => void> = [];
    const calls = stub(() => new Promise((resolve) => replies.push(resolve)));
    setup();
    const input = await openPalette();
    await fireEvent.input(input, { target: { value: 'zzza' } });
    await fireEvent.input(input, { target: { value: 'zzzb' } });
    expect(calls[0].signal?.aborted).toBe(true);
    expect(calls[1].signal?.aborted).toBe(false);
    replies[1](page({ object: makeObj(2, { title: 'fresh' }), snippet: 'fresh', score: 1 }));
    await waitFor(() => expect(options()).toHaveLength(1));
    replies[0](page({ object: makeObj(1, { title: 'stale' }), snippet: 'stale', score: 1 }));
    await new Promise((r) => setTimeout(r, 10));
    expect(options().map((o) => o.textContent)).toEqual([expect.stringContaining('fresh')]);
  });

  it('arrows move the selection and Enter on a result opens it', async () => {
    stub(async () =>
      page(
        { object: makeObj(3, { type: 'view', title: 'zzz view' }), snippet: 'zzz view', score: 2 },
        { object: makeObj(4, { title: 'zzz note' }), snippet: 'zzz note', score: 1 },
      ),
    );
    const other = setup();
    const input = await openPalette();
    await fireEvent.input(input, { target: { value: 'zzz' } });
    await waitFor(() => expect(options()).toHaveLength(2));
    await fireEvent.keyDown(input, { key: 'ArrowDown' });
    expect(selected()).toHaveTextContent('zzz note');
    await fireEvent.keyDown(input, { key: 'ArrowDown' });
    expect(selected()).toHaveTextContent('zzz view');
    await fireEvent.keyDown(input, { key: 'ArrowUp' });
    expect(selected()).toHaveTextContent('zzz note');
    await fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(location.hash).toBe('#/o/4'));
    expect(dialog()).toBeNull();
    expect(other).toHaveFocus();

    const again = await openPalette();
    await fireEvent.input(again, { target: { value: 'zzz' } });
    await waitFor(() => expect(options()).toHaveLength(2));
    await fireEvent.keyDown(again, { key: 'Enter' });
    await waitFor(() => expect(location.hash).toBe('#/v/3'));
  });

  it('Enter on a command runs it; a pressed row runs too', async () => {
    stub();
    setup();
    const input = await openPalette();
    await fireEvent.input(input, { target: { value: 'trash' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(location.hash).toBe('#/trash'));

    await openPalette();
    await fireEvent.mouseDown(screen.getByRole('option', { name: 'go to upcoming' }));
    await waitFor(() => expect(location.hash).toBe('#/upcoming'));
  });

  it('new task closes the palette and dispatches capture-open', async () => {
    stub();
    setup();
    const opened = vi.fn();
    window.addEventListener(CAPTURE_OPEN, opened);
    const input = await openPalette();
    await fireEvent.input(input, { target: { value: 'new' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    await waitFor(() => expect(opened).toHaveBeenCalledTimes(1));
    expect(dialog()).toBeNull();
    window.removeEventListener(CAPTURE_OPEN, opened);
  });

  it('Tab keeps focus in the input', async () => {
    stub();
    setup();
    const input = await openPalette();
    for (const shiftKey of [false, true]) {
      const tab = new KeyboardEvent('keydown', { key: 'Tab', shiftKey, bubbles: true, cancelable: true });
      input.dispatchEvent(tab);
      expect(tab.defaultPrevented).toBe(true);
      expect(input).toHaveFocus();
    }
  });

  it('the palette-open event opens it with its text and searches it', async () => {
    const calls = stub();
    setup();
    window.dispatchEvent(new CustomEvent(PALETTE_OPEN, { detail: { text: 'upc' } }));
    const input = (await screen.findByRole('combobox', { name: 'palette' })) as HTMLInputElement;
    await waitFor(() => expect(input).toHaveFocus());
    expect(input.value).toBe('upc');
    expect(options().map((o) => o.textContent?.trim())).toEqual(['go to upcoming']);
    expect(calls.map((c) => c.url)).toEqual(['/api/search?q=upc&limit=10']);
  });

  it('Ctrl+K while the capture bar is open closes the bar first; Esc returns where the bar was opened', async () => {
    stub();
    const other = setup();
    render(CaptureBar);
    const capture = () => screen.queryByRole('dialog', { name: 'capture' });
    await fireEvent.keyDown(other, { key: ' ', code: 'Space', ctrlKey: true });
    const bar = await screen.findByRole('textbox', { name: 'capture' });
    await waitFor(() => expect(bar).toHaveFocus());
    const input = await openPalette(bar);
    expect(capture()).toBeNull();
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
    await fireEvent.keyDown(input, { key: 'Escape' });
    await waitFor(() => expect(dialog()).toBeNull());
    expect(other).toHaveFocus();
  });

  it('> in the capture bar still hands its text to the palette, leaving one dialog', async () => {
    stub();
    const other = setup();
    render(CaptureBar);
    await fireEvent.keyDown(other, { key: ' ', code: 'Space', ctrlKey: true });
    const bar = await screen.findByRole('textbox', { name: 'capture' });
    await fireEvent.input(bar, { target: { value: '>upc' } });
    const input = (await screen.findByRole('combobox', { name: 'palette' })) as HTMLInputElement;
    await waitFor(() => expect(input).toHaveFocus());
    expect(input.value).toBe('upc');
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
    await fireEvent.keyDown(input, { key: 'Escape' });
    await waitFor(() => expect(dialog()).toBeNull());
    expect(other).toHaveFocus();
  });
});
