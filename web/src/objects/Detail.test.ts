import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import Detail from './Detail.svelte';
import { parseProps, parseValue } from './PropsEditor.svelte';
import { makeObj, stubApi, type Route } from './testing';
import type { Obj } from '../lib/objects';

/** Serves `obj` on GET and answers each PATCH with `reply(patch)` (default: the patch merged in). */
function api(obj: Obj, more: Record<string, Route> = {}, reply?: (patch: Partial<Obj>) => Response) {
  const patches: unknown[] = [];
  const fetch = stubApi({
    'GET /api/objects/7': () => Response.json(obj),
    'PATCH /api/objects/7': (_url, init) => {
      const patch = JSON.parse(String(init?.body)) as Partial<Obj>;
      patches.push(patch);
      return reply ? reply(patch) : Response.json({ ...obj, ...patch });
    },
    ...more,
  });
  return { fetch, patches };
}

const bad = (error: string) => () => Response.json({ error }, { status: 400 });

async function open(ondeleted = () => {}) {
  render(Detail, { props: { id: 7, ondeleted } });
  await screen.findByRole('heading');
}

beforeEach(() => {
  location.hash = '#/o/7';
});
afterEach(() => vi.unstubAllGlobals());

describe('detail', () => {
  it('shows the object fields and properties', async () => {
    const obj = makeObj(7, {
      title: 'Groceries',
      pinned: true,
      tags: ['shop/food', 'home'],
      properties: { aisle: 3, store: 'corner', list: ['milk', 1] },
      updated: '2026-10-30T10:00:00Z',
    });
    api(obj);
    await open();
    expect(screen.getByRole('heading')).toHaveTextContent('* Groceries');
    const field = (name: string) => screen.getByText(name, { selector: 'dt' }).nextElementSibling?.textContent;
    expect(field('type')).toBe('note');
    expect(field('id')).toBe('7');
    expect(screen.getByRole('button', { name: 'remove tag shop/food' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'remove tag home' })).toBeInTheDocument();
    expect(field('pinned')).toBe('true');
    expect(field('archived')).toBe('false');
    expect(field('created')).toBe('2026-10-29T09:15:00Z');
    expect(field('updated')).toBe('2026-10-30T10:00:00Z');
    expect(screen.getByText('aisle: 3')).toBeInTheDocument();
    expect(screen.getByText('store: corner')).toBeInTheDocument();
    expect(screen.getByText('list: ["milk",1]')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'panels' })).toBeInTheDocument();
  });

  it('shows object not found on 404', async () => {
    stubApi({ 'GET /api/objects/7': () => Response.json({ error: 'object not found' }, { status: 404 }) });
    render(Detail, { props: { id: 7, ondeleted: () => {} } });
    expect(await screen.findByRole('alert')).toHaveTextContent(/^object not found$/);
  });

  it('shows other failures with their cause', async () => {
    stubApi({ 'GET /api/objects/7': () => new Response('', { status: 500 }) });
    render(Detail, { props: { id: 7, ondeleted: () => {} } });
    expect(await screen.findByRole('alert')).toHaveTextContent('error: /api/objects/7: HTTP 500');
  });

  it('Esc returns to the list', async () => {
    api(makeObj(7));
    await open();
    await fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(location.hash).toBe('#/');
  });
});

describe('title', () => {
  it('Enter saves the edited title with one PATCH', async () => {
    const { patches } = api(makeObj(7, { title: 'Old' }));
    await open();
    await fireEvent.click(screen.getByRole('button', { name: 'Old' }));
    const input = screen.getByRole('textbox', { name: 'title' });
    expect(input).toHaveFocus();
    await fireEvent.input(input, { target: { value: 'New' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(await screen.findByRole('heading', { name: 'New' })).toBeInTheDocument();
    expect(patches).toEqual([{ title: 'New' }]);
  });

  it('Esc cancels without a request and stays on the page', async () => {
    const { patches } = api(makeObj(7, { title: 'Old' }));
    await open();
    await fireEvent.click(screen.getByRole('button', { name: 'Old' }));
    const input = screen.getByRole('textbox', { name: 'title' });
    await fireEvent.input(input, { target: { value: 'New' } });
    await fireEvent.keyDown(input, { key: 'Escape' });
    expect(screen.getByRole('heading', { name: 'Old' })).toBeInTheDocument();
    expect(patches).toEqual([]);
    expect(location.hash).toBe('#/o/7');
  });

  it('shows the server 400 text inline and keeps editing', async () => {
    api(makeObj(7, { title: 'Old' }), { 'PATCH /api/objects/7': bad('title must be 1 to 1000 characters') });
    await open();
    await fireEvent.click(screen.getByRole('button', { name: 'Old' }));
    const input = screen.getByRole('textbox', { name: 'title' });
    await fireEvent.input(input, { target: { value: ' ' } });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(await screen.findByRole('alert')).toHaveTextContent('title must be 1 to 1000 characters');
    expect(screen.getByRole('textbox', { name: 'title' })).toBeInTheDocument();
  });
});

describe('tags', () => {
  it('adds a tag and shows the normalized tags from the server', async () => {
    const { patches } = api(makeObj(7, { tags: ['home'] }), {}, () =>
      Response.json(makeObj(7, { tags: ['home', 'work/x'] })),
    );
    await open();
    const input = screen.getByRole('textbox', { name: 'add tag' });
    await fireEvent.input(input, { target: { value: ' Work/X ' } });
    await fireEvent.submit(input);
    expect(await screen.findByRole('button', { name: 'remove tag work/x' })).toBeInTheDocument();
    expect(patches).toEqual([{ tags: ['home', 'Work/X'] }]);
    expect(input).toHaveValue('');
  });

  it('removes a tag', async () => {
    const { patches } = api(makeObj(7, { tags: ['home', 'work'] }));
    await open();
    await fireEvent.click(screen.getByRole('button', { name: 'remove tag home' }));
    await waitFor(() => expect(screen.queryByRole('button', { name: 'remove tag home' })).not.toBeInTheDocument());
    expect(patches).toEqual([{ tags: ['work'] }]);
    expect(screen.getByRole('button', { name: 'remove tag work' })).toBeInTheDocument();
  });

  it('shows the server 400 text inline', async () => {
    api(makeObj(7), { 'PATCH /api/objects/7': bad('tag segments must not be empty') });
    await open();
    const input = screen.getByRole('textbox', { name: 'add tag' });
    await fireEvent.input(input, { target: { value: 'a//b' } });
    await fireEvent.submit(input);
    expect(await screen.findByRole('alert')).toHaveTextContent('tag segments must not be empty');
    expect(input).toHaveValue('a//b');
  });
});

describe('properties', () => {
  async function editProps(text: string) {
    await fireEvent.click(screen.getByRole('button', { name: '[ edit properties ]' }));
    const area = screen.getByRole('textbox', { name: 'properties as key: value lines' });
    await fireEvent.input(area, { target: { value: text } });
    return area;
  }

  it('saves key: value lines with CLI value parsing in one PATCH', async () => {
    const { patches } = api(makeObj(7, { properties: { list: ['milk', 1], code: '42' } }));
    await open();
    await fireEvent.click(screen.getByRole('button', { name: '[ edit properties ]' }));
    const area = screen.getByRole('textbox', { name: 'properties as key: value lines' });
    expect(area).toHaveValue('list: ["milk",1]\ncode: 42');
    await fireEvent.input(area, {
      target: { value: 'list: ["milk",1]\ncode: 42\n\naisle: 3\nprice: 2.5\ndone: true\nstore:  corner shop ' },
    });
    await fireEvent.click(screen.getByRole('button', { name: '[ save ]' }));
    expect(await screen.findByText('aisle: 3')).toBeInTheDocument();
    expect(patches).toEqual([
      { properties: { list: ['milk', 1], code: '42', aisle: 3, price: 2.5, done: true, store: 'corner shop' } },
    ]);
  });

  it('shows a parse error inline without sending', async () => {
    const { patches } = api(makeObj(7));
    await open();
    await editProps('aisle: 3\njust words');
    await fireEvent.click(screen.getByRole('button', { name: '[ save ]' }));
    expect(screen.getByRole('alert')).toHaveTextContent('line 2: expected key: value');
    expect(patches).toEqual([]);
  });

  it('shows the server 400 text inline', async () => {
    api(makeObj(7), { 'PATCH /api/objects/7': bad('properties must have at most 100 keys') });
    await open();
    const area = await editProps('a: 1');
    await fireEvent.keyDown(area, { key: 'Enter', ctrlKey: true });
    expect(await screen.findByRole('alert')).toHaveTextContent('properties must have at most 100 keys');
  });

  it('Esc cancels editing without leaving the page', async () => {
    const { patches } = api(makeObj(7, { properties: { a: 1 } }));
    await open();
    const area = await editProps('a: 2');
    await fireEvent.keyDown(area, { key: 'Escape' });
    expect(screen.getByText('a: 1')).toBeInTheDocument();
    expect(patches).toEqual([]);
    expect(location.hash).toBe('#/o/7');
  });
});

describe('parseProps', () => {
  it('parses values like the CLI', () => {
    expect([parseValue('3'), parseValue('-1.5e2'), parseValue('false'), parseValue('1e999'), parseValue('')]).toEqual([
      3,
      -150,
      false,
      '1e999',
      '',
    ]);
  });

  it('rejects a missing key and duplicates', () => {
    expect(parseProps(': 1', {})).toEqual({ error: 'line 1: missing key' });
    expect(parseProps('a: 1\na: 2', {})).toEqual({ error: 'line 2: duplicate key "a"' });
  });
});

describe('actions', () => {
  it('pin and archive toggle with one PATCH each', async () => {
    const { patches } = api(makeObj(7, { title: 'T' }));
    await open();
    await fireEvent.click(screen.getByRole('button', { name: '[ pin ]' }));
    expect(await screen.findByRole('button', { name: '[ unpin ]' })).toBeInTheDocument();
    expect(screen.getByRole('heading')).toHaveTextContent('* T');
    await fireEvent.click(screen.getByRole('button', { name: '[ archive ]' }));
    expect(await screen.findByRole('button', { name: '[ unarchive ]' })).toBeInTheDocument();
    expect(patches).toEqual([{ pinned: true }, { archived: true }]);
  });

  it('unpin and unarchive send false', async () => {
    const { patches } = api(makeObj(7, { pinned: true, archived: true }));
    await open();
    await fireEvent.click(screen.getByRole('button', { name: '[ unpin ]' }));
    await screen.findByRole('button', { name: '[ pin ]' });
    await fireEvent.click(screen.getByRole('button', { name: '[ unarchive ]' }));
    await screen.findByRole('button', { name: '[ archive ]' });
    expect(patches).toEqual([{ pinned: false }, { archived: false }]);
  });

  it('delete sends DELETE and calls ondeleted', async () => {
    const { fetch } = api(makeObj(7), { 'DELETE /api/objects/7': () => new Response(null, { status: 204 }) });
    const ondeleted = vi.fn();
    await open(ondeleted);
    await fireEvent.click(screen.getByRole('button', { name: '[ delete ]' }));
    await waitFor(() => expect(ondeleted).toHaveBeenCalledOnce());
    expect(fetch).toHaveBeenCalledWith('/api/objects/7', expect.objectContaining({ method: 'DELETE' }));
  });

  it('a failed delete shows the cause and stays', async () => {
    api(makeObj(7), { 'DELETE /api/objects/7': () => new Response('', { status: 500 }) });
    const ondeleted = vi.fn();
    await open(ondeleted);
    await fireEvent.click(screen.getByRole('button', { name: '[ delete ]' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('/api/objects/7: HTTP 500');
    expect(ondeleted).not.toHaveBeenCalled();
  });
});
