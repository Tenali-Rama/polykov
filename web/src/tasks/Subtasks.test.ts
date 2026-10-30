import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { stubApi } from '../objects/testing';
import type { Row } from '../lib/tasks';
import Subtasks from './Subtasks.svelte';
import { makeRow } from './testing';

afterEach(() => vi.unstubAllGlobals());

function api(rows: Row[], created = 201) {
  const posts: unknown[] = [];
  stubApi({
    'GET /api/tasks/7/subtasks': () => Response.json({ tasks: rows }),
    'POST /api/tasks': (_url, init) => {
      const body = JSON.parse(String(init?.body)) as { title: string };
      posts.push(body);
      if (created !== 201) return Response.json({ error: 'title must be 1 to 1000 characters' }, { status: created });
      rows = [...rows, makeRow(20 + rows.length, {}, { title: body.title })];
      return Response.json(rows[rows.length - 1], { status: 201 });
    },
  });
  return posts;
}

describe('sub-tasks', () => {
  it('lists children as task rows', async () => {
    api([makeRow(8, { status: 'done' }, { title: 'child a' }), makeRow(9, {}, { title: 'child b' })]);
    render(Subtasks, { props: { id: 7 } });
    expect(await screen.findByRole('link', { name: 'child b' })).toHaveAttribute('href', '#/o/9');
    expect(screen.getByRole('button', { name: 'reopen child a' })).toHaveTextContent('[x]');
  });

  it('Enter in the prompt adds a sub-task with parent_id and reloads', async () => {
    const posts = api([]);
    render(Subtasks, { props: { id: 7 } });
    expect(await screen.findByText('no sub-tasks')).toBeInTheDocument();
    const input = screen.getByRole('textbox', { name: 'add sub-task' });
    await fireEvent.submit(input);
    await fireEvent.input(input, { target: { value: ' buy paint ' } });
    await fireEvent.submit(input);
    expect(await screen.findByRole('link', { name: 'buy paint' })).toBeInTheDocument();
    expect(posts).toEqual([{ title: 'buy paint', task: { parent_id: 7 } }]);
    expect(input).toHaveValue('');
  });

  it('a failed add shows the server text and keeps the draft', async () => {
    api([], 400);
    render(Subtasks, { props: { id: 7 } });
    const input = await screen.findByRole('textbox', { name: 'add sub-task' });
    await fireEvent.input(input, { target: { value: 'x' } });
    await fireEvent.submit(input);
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('title must be 1 to 1000 characters'));
    expect(input).toHaveValue('x');
  });
});
