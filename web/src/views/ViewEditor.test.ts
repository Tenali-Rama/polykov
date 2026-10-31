import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { makeObj, stubApi } from '../objects/testing';
import type { View } from '../lib/views';
import ViewEditor from './ViewEditor.svelte';

const view = (over: Partial<View> = {}): View => ({
  ...makeObj(4, { type: 'view', title: 'Work' }),
  properties: { query: 'tag:work', layout: 'list', group: 'none', sort: 'smart' },
  ...over,
});
const body = (init?: RequestInit) => JSON.parse(String(init?.body));

afterEach(() => vi.unstubAllGlobals());

function setup(props: { view?: View } = {}) {
  const onsaved = vi.fn();
  const oncancel = vi.fn();
  render(ViewEditor, { ...props, onsaved, oncancel });
  return { onsaved, oncancel };
}

describe('ViewEditor', () => {
  it('creates a view with the query and the chosen settings', async () => {
    const created = view({ id: 9 });
    const fetch = stubApi({ 'POST /api/views': () => Response.json(created, { status: 201 }) });
    const { onsaved } = setup();
    expect(screen.getByLabelText('title')).toHaveFocus();
    await fireEvent.input(screen.getByLabelText('title'), { target: { value: 'Urgent' } });
    await fireEvent.input(screen.getByLabelText('query'), { target: { value: 'type:task priority:<=2' } });
    await fireEvent.change(screen.getByLabelText('group'), { target: { value: 'priority' } });
    await fireEvent.change(screen.getByLabelText('sort'), { target: { value: 'deadline' } });
    await fireEvent.click(screen.getByRole('button', { name: '[ save ]' }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledWith(created));
    expect(body(fetch.mock.calls[0][1])).toEqual({
      title: 'Urgent',
      query: 'type:task priority:<=2',
      layout: 'list',
      group: 'priority',
      sort: 'deadline',
    });
  });

  it('edits an existing view through PATCH, starting from its settings', async () => {
    const fetch = stubApi({ 'PATCH /api/views/4': (_, init) => Response.json(view(body(init))) });
    const { onsaved } = setup({ view: view() });
    expect(screen.getByLabelText('query')).toHaveValue('tag:work');
    await fireEvent.change(screen.getByLabelText('layout'), { target: { value: 'kanban' } });
    await fireEvent.submit(screen.getByRole('form', { name: 'edit view' }));
    await waitFor(() => expect(onsaved).toHaveBeenCalled());
    expect(body(fetch.mock.calls[0][1])).toMatchObject({ title: 'Work', query: 'tag:work', layout: 'kanban' });
  });

  it("shows the server's 400 inline, marks the error offset under the query, and keeps the input", async () => {
    stubApi({
      'POST /api/views': () => Response.json({ error: 'query: expected a value after priority:', at: 5 }, { status: 400 }),
    });
    const { onsaved } = setup();
    await fireEvent.input(screen.getByLabelText('title'), { target: { value: 'Bad' } });
    await fireEvent.input(screen.getByLabelText('query'), { target: { value: 'tag:x priority:' } });
    await fireEvent.click(screen.getByRole('button', { name: '[ save ]' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('error: query: expected a value after priority:');
    expect(screen.getByText(/\^$/).textContent).toBe('     ^');
    expect(screen.getByLabelText('query')).toHaveValue('tag:x priority:');
    expect(screen.getByLabelText('query')).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByLabelText('title')).toHaveValue('Bad');
    expect(onsaved).not.toHaveBeenCalled();
  });

  it('an error without an offset shows no marker, and the next save clears it', async () => {
    let fail = true;
    stubApi({
      'POST /api/views': () =>
        fail ? Response.json({ error: 'title must not be empty' }, { status: 400 }) : Response.json(view()),
    });
    const { onsaved } = setup();
    await fireEvent.click(screen.getByRole('button', { name: '[ save ]' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('error: title must not be empty');
    expect(screen.queryByText(/\^$/)).toBeNull();
    fail = false;
    await fireEvent.click(screen.getByRole('button', { name: '[ save ]' }));
    await waitFor(() => expect(onsaved).toHaveBeenCalled());
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('Esc cancels and is marked handled; [ cancel ] cancels too', async () => {
    stubApi({});
    const { oncancel } = setup();
    const outer = vi.fn();
    window.addEventListener('keydown', outer);
    await fireEvent.keyDown(screen.getByLabelText('query'), { key: 'Escape' });
    window.removeEventListener('keydown', outer);
    expect(oncancel).toHaveBeenCalledTimes(1);
    expect(outer).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: '[ cancel ]' }));
    expect(oncancel).toHaveBeenCalledTimes(2);
  });
});
