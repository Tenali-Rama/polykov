import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { stubApi, type Route } from '../objects/testing';
import type { ChecklistItem } from '../lib/tasks';
import Checklist from './Checklist.svelte';
import { makeRow } from './testing';

const item = (text: string, done = false): ChecklistItem => ({ text, done });

/** Serves task 7 with `items`; each PATCH replaces the checklist and is recorded. */
function api(items: ChecklistItem[], patch?: Route) {
  let current = items;
  const sent: ChecklistItem[][] = [];
  stubApi({
    'GET /api/tasks/7': () => Response.json(makeRow(7, { checklist: current })),
    'PATCH /api/tasks/7': (url, init) => {
      if (patch) return patch(url, init);
      current = (JSON.parse(String(init?.body)) as { task: { checklist: ChecklistItem[] } }).task.checklist;
      sent.push(current);
      return Response.json(makeRow(7, { checklist: current }));
    },
  });
  return sent;
}

async function open(items: ChecklistItem[], patch?: Route) {
  const sent = api(items, patch);
  render(Checklist, { props: { id: 7 } });
  await screen.findByRole('textbox', { name: 'add item' });
  return sent;
}

const itemButton = (text: string) => screen.getByRole('button', { name: new RegExp(`^\\[[ x]\\] ${text}$`) });

afterEach(() => vi.unstubAllGlobals());

describe('checklist', () => {
  it('shows items as [ ] and [x]', async () => {
    await open([item('draft'), item('send', true)]);
    expect(itemButton('draft')).toHaveTextContent('[ ] draft');
    expect(itemButton('send')).toHaveTextContent('[x] send');
    expect(itemButton('send')).toHaveAttribute('aria-pressed', 'true');
  });

  it('a click toggles an item, sends the whole list, and keeps focus on it', async () => {
    const sent = await open([item('a'), item('b')]);
    await fireEvent.click(itemButton('b'));
    await waitFor(() => expect(itemButton('b')).toHaveTextContent('[x] b'));
    expect(sent).toEqual([[item('a'), item('b', true)]]);
    expect(itemButton('b')).toHaveFocus();
  });

  it('[ edit ]: Enter saves, Esc cancels with preventDefault', async () => {
    const sent = await open([item('a'), item('b')]);
    await fireEvent.click(screen.getByRole('button', { name: 'edit a' }));
    const input = screen.getByRole('textbox', { name: 'edit item 1' });
    expect(input).toHaveFocus();
    await fireEvent.input(input, { target: { value: 'a2' } });
    const esc = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    input.dispatchEvent(esc);
    expect(esc.defaultPrevented).toBe(true);
    await waitFor(() => expect(screen.getByRole('button', { name: 'edit a' })).toHaveFocus());
    expect(sent).toEqual([]);

    await fireEvent.click(screen.getByRole('button', { name: 'edit a' }));
    const again = screen.getByRole('textbox', { name: 'edit item 1' });
    await fireEvent.input(again, { target: { value: 'a2' } });
    await fireEvent.keyDown(again, { key: 'Enter' });
    await waitFor(() => expect(screen.getByRole('button', { name: 'edit a2' })).toHaveFocus());
    expect(sent).toEqual([[item('a2'), item('b')]]);
  });

  it('a rejected edit shows the server text and keeps the editor', async () => {
    await open([item('a')], () => Response.json({ error: 'checklist text must be 1 to 500 characters' }, { status: 400 }));
    await fireEvent.click(screen.getByRole('button', { name: 'edit a' }));
    const input = screen.getByRole('textbox', { name: 'edit item 1' });
    await fireEvent.input(input, { target: { value: '   ' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(await screen.findByRole('alert')).toHaveTextContent('checklist text must be 1 to 500 characters');
    expect(screen.getByRole('textbox', { name: 'edit item 1' })).toHaveValue('   ');
  });

  it('Alt+ArrowDown/Up and [ up ]/[ down ] reorder, focus follows the item', async () => {
    const sent = await open([item('a'), item('b'), item('c')]);
    await fireEvent.keyDown(itemButton('a'), { key: 'ArrowDown', altKey: true });
    await waitFor(() => expect(itemButton('a')).toHaveFocus());
    await fireEvent.keyDown(itemButton('a'), { key: 'ArrowDown' });
    await fireEvent.click(screen.getByRole('button', { name: 'move up c' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'move up c' })).toHaveFocus());
    // Up to the top: its [ up ] is disabled there, so the item keeps focus.
    await fireEvent.click(screen.getByRole('button', { name: 'move up c' }));
    await waitFor(() => expect(itemButton('c')).toHaveFocus());
    await fireEvent.keyDown(itemButton('c'), { key: 'ArrowUp', altKey: true });
    expect(screen.getByRole('button', { name: 'move up c' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'move down a' })).toBeDisabled();
    expect(sent).toEqual([
      [item('b'), item('a'), item('c')],
      [item('b'), item('c'), item('a')],
      [item('c'), item('b'), item('a')],
    ]);
  });

  it('[ delete ] removes an item and focuses the next one, else the add prompt', async () => {
    const sent = await open([item('a'), item('b')]);
    await fireEvent.click(screen.getByRole('button', { name: 'delete a' }));
    await waitFor(() => expect(itemButton('b')).toHaveFocus());
    await fireEvent.click(screen.getByRole('button', { name: 'delete b' }));
    await waitFor(() => expect(screen.getByRole('textbox', { name: 'add item' })).toHaveFocus());
    expect(sent).toEqual([[item('b')], []]);
    expect(screen.getByText('no items')).toBeInTheDocument();
  });

  it('the add prompt adds with Enter and stays focused; blank adds nothing', async () => {
    const sent = await open([item('a')]);
    const input = screen.getByRole('textbox', { name: 'add item' });
    await fireEvent.submit(input);
    await fireEvent.input(input, { target: { value: 'b' } });
    await fireEvent.submit(input);
    await waitFor(() => expect(input).toHaveValue(''));
    expect(itemButton('b')).toBeInTheDocument();
    expect(input).toHaveFocus();
    expect(sent).toEqual([[item('a'), item('b')]]);
  });
});
