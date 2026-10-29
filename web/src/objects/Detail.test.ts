import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import Detail from './Detail.svelte';
import { makeObj, stubApi } from './testing';

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
    stubApi({ 'GET /api/objects/7': () => Response.json(obj) });
    render(Detail, { props: { id: 7 } });
    expect(await screen.findByRole('heading')).toHaveTextContent('* Groceries');
    const field = (name: string) => screen.getByText(name, { selector: 'dt' }).nextElementSibling?.textContent;
    expect(field('type')).toBe('note');
    expect(field('id')).toBe('7');
    expect(field('tags')).toBe('#shop/food #home');
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
    render(Detail, { props: { id: 7 } });
    expect(await screen.findByRole('alert')).toHaveTextContent(/^object not found$/);
  });

  it('shows other failures with their cause', async () => {
    stubApi({ 'GET /api/objects/7': () => new Response('', { status: 500 }) });
    render(Detail, { props: { id: 7 } });
    expect(await screen.findByRole('alert')).toHaveTextContent('error: /api/objects/7: HTTP 500');
  });

  it('Esc returns to the list', async () => {
    stubApi({ 'GET /api/objects/7': () => Response.json(makeObj(7)) });
    render(Detail, { props: { id: 7 } });
    await screen.findByRole('heading');
    await fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(location.hash).toBe('#/');
  });
});
