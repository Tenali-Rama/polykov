import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import Comments from './Comments.svelte';
import { stubApi, type Route } from './testing';
import type { Comment } from '../lib/comments';

function comment(id: number, body: string, created = '2026-10-29T09:15:00Z'): Comment {
  return { id, object_id: 7, body, created, updated: created };
}

const bad = () => Response.json({ error: 'body must be 1 to 10000 characters' }, { status: 400 });

function api(comments: Comment[], more: Record<string, Route> = {}) {
  const bodies: unknown[] = [];
  const record = (init?: RequestInit) => {
    const body = JSON.parse(String(init?.body)) as { body: string };
    bodies.push(body);
    return body.body;
  };
  const fetch = stubApi({
    'GET /api/objects/7/comments': () => Response.json({ comments }),
    'POST /api/objects/7/comments': (_url, init) => Response.json(comment(9, record(init), '2026-10-30T08:00:00Z'), { status: 201 }),
    'PATCH /api/comments/1': (_url, init) => Response.json(comment(1, record(init))),
    'DELETE /api/comments/1': () => new Response(null, { status: 204 }),
    ...more,
  });
  return { fetch, bodies };
}

async function open() {
  render(Comments, { props: { id: 7 } });
  return screen.findByRole('textbox', { name: 'add comment' });
}

const items = () => within(screen.getByRole('region', { name: 'comments' })).getAllByRole('listitem');

afterEach(() => vi.unstubAllGlobals());

describe('comments panel', () => {
  it('lists comments oldest first with their created time', async () => {
    api([comment(1, 'first', '2026-10-29T09:15:00Z'), comment(2, 'second', '2026-10-29T10:00:00Z')]);
    await open();
    await screen.findByText('first');
    const [a, b] = items();
    expect(a).toHaveTextContent('2026-10-29T09:15:00Z');
    expect(a).toHaveTextContent('first');
    expect(b).toHaveTextContent('2026-10-29T10:00:00Z');
    expect(b).toHaveTextContent('second');
    expect(within(a).getByRole('button', { name: 'edit comment 1' })).toHaveTextContent('[ edit ]');
    expect(within(a).getByRole('button', { name: 'delete comment 1' })).toHaveTextContent('[ delete ]');
  });

  it('shows no comments when empty', async () => {
    api([]);
    await open();
    expect(await screen.findByText('no comments')).toBeInTheDocument();
  });

  it('Ctrl+Enter adds a comment at the end and clears the prompt', async () => {
    const { bodies } = api([comment(1, 'first')]);
    const input = await open();
    await screen.findByText('first');
    await fireEvent.input(input, { target: { value: 'second' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(bodies).toEqual([]);
    await fireEvent.keyDown(input, { key: 'Enter', ctrlKey: true });
    expect(await screen.findByText('second')).toBeInTheDocument();
    expect(bodies).toEqual([{ body: 'second' }]);
    expect(items()[1]).toHaveTextContent('2026-10-30T08:00:00Z');
    expect(input).toHaveValue('');
  });

  it('[ add ] adds too', async () => {
    const { bodies } = api([]);
    const input = await open();
    await fireEvent.input(input, { target: { value: 'tapped' } });
    await fireEvent.click(screen.getByRole('button', { name: '[ add ]' }));
    expect(await screen.findByText('tapped')).toBeInTheDocument();
    expect(bodies).toEqual([{ body: 'tapped' }]);
  });

  it('shows the server 400 text inline and keeps the draft', async () => {
    api([], { 'POST /api/objects/7/comments': bad });
    const input = await open();
    await fireEvent.input(input, { target: { value: '  ' } });
    await fireEvent.keyDown(input, { key: 'Enter', ctrlKey: true });
    expect(await screen.findByRole('alert')).toHaveTextContent('body must be 1 to 10000 characters');
    expect(input).toHaveValue('  ');
  });

  it('Esc in a non-empty prompt clears it and marks the key handled; an empty prompt lets it through', async () => {
    const { bodies } = api([]);
    const input = await open();
    await fireEvent.input(input, { target: { value: 'unsent draft' } });
    const esc = () => new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    const first = esc();
    input.dispatchEvent(first);
    expect(first.defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(input).toHaveValue(''));
    const second = esc();
    input.dispatchEvent(second);
    expect(second.defaultPrevented).toBe(false);
    expect(bodies).toEqual([]);
  });

  it('edits inline: Ctrl+Enter saves with one PATCH', async () => {
    const { bodies } = api([comment(1, 'frist')]);
    await open();
    await fireEvent.click(await screen.findByRole('button', { name: 'edit comment 1' }));
    const area = screen.getByRole('textbox', { name: 'edit comment 1' });
    expect(area).toHaveFocus();
    expect(area).toHaveValue('frist');
    await fireEvent.input(area, { target: { value: 'first' } });
    await fireEvent.keyDown(area, { key: 'Enter', ctrlKey: true });
    expect(await screen.findByText('first')).toBeInTheDocument();
    expect(screen.queryByRole('textbox', { name: 'edit comment 1' })).not.toBeInTheDocument();
    expect(bodies).toEqual([{ body: 'first' }]);
  });

  it('Esc cancels an edit without a request, marking the key handled', async () => {
    const { bodies } = api([comment(1, 'keep')]);
    await open();
    await fireEvent.click(await screen.findByRole('button', { name: 'edit comment 1' }));
    const area = screen.getByRole('textbox', { name: 'edit comment 1' });
    await fireEvent.input(area, { target: { value: 'changed' } });
    const esc = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    area.dispatchEvent(esc);
    expect(esc.defaultPrevented).toBe(true);
    expect(await screen.findByText('keep')).toBeInTheDocument();
    expect(bodies).toEqual([]);
  });

  it('a 400 on edit shows inline and keeps editing', async () => {
    api([comment(1, 'keep')], { 'PATCH /api/comments/1': bad });
    await open();
    await fireEvent.click(await screen.findByRole('button', { name: 'edit comment 1' }));
    const area = screen.getByRole('textbox', { name: 'edit comment 1' });
    await fireEvent.input(area, { target: { value: '' } });
    await fireEvent.keyDown(area, { key: 'Enter', ctrlKey: true });
    expect(await screen.findByRole('alert')).toHaveTextContent('body must be 1 to 10000 characters');
    expect(screen.getByRole('textbox', { name: 'edit comment 1' })).toBeInTheDocument();
  });

  it('[ delete ] sends DELETE and removes the comment', async () => {
    const { fetch } = api([comment(1, 'gone'), comment(2, 'stays')]);
    await open();
    await fireEvent.click(await screen.findByRole('button', { name: 'delete comment 1' }));
    await vi.waitFor(() => expect(screen.queryByText('gone')).not.toBeInTheDocument());
    expect(screen.getByText('stays')).toBeInTheDocument();
    expect(fetch).toHaveBeenCalledWith('/api/comments/1', expect.objectContaining({ method: 'DELETE' }));
  });

  it('a failed delete shows the cause and keeps the comment', async () => {
    api([comment(1, 'stays')], { 'DELETE /api/comments/1': () => Response.json({ error: 'comment not found' }, { status: 404 }) });
    await open();
    await fireEvent.click(await screen.findByRole('button', { name: 'delete comment 1' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('comment not found');
    expect(screen.getByText('stays')).toBeInTheDocument();
  });
});
