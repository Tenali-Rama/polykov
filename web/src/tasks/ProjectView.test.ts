import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { makeObj, stubApi, type Route } from '../objects/testing';
import type { Project, Section } from '../lib/projects';
import ProjectView from './ProjectView.svelte';
import { makeRow } from './testing';

const project = (over: Partial<Project['project']> = {}): Project => ({
  ...makeObj(2, { type: 'project', title: 'Launch' }),
  project: { status: 'active', area_id: null, start: '2026-11-01', deadline: null, open_tasks: 3, done_tasks: 0, ...over },
});
const section = (id: number, title: string, position: number): Section => ({ id, project_id: 2, title, position });

let sections: Section[];
const body = (init?: RequestInit) => JSON.parse(String(init?.body));

function api(more: Record<string, Route> = {}) {
  sections = [section(10, 'Plan', 0), section(11, 'Build', 1)];
  return stubApi({
    'GET /api/projects/2': () => Response.json(project()),
    'GET /api/projects/2/sections': () => Response.json({ sections }),
    'GET /api/objects': () => Response.json({ objects: [makeObj(9, { type: 'area', title: 'Work' })], total: 1 }),
    'GET /api/lists/all': () =>
      Response.json({
        tasks: [makeRow(1, { section_id: 11 }), makeRow(2), makeRow(3, { section_id: 10 })],
        total: 3,
      }),
    ...more,
  });
}

const group = (name: string) => within(screen.getByRole('list', { name }));

afterEach(() => vi.unstubAllGlobals());

async function view(more: Record<string, Route> = {}) {
  const fetch = api(more);
  render(ProjectView, { id: 2, kind: 'project' });
  await screen.findByRole('heading', { name: 'Launch' });
  return fetch;
}

describe('project', () => {
  it('shows title, status, area, dates, then unsectioned and each section with its tasks', async () => {
    const fetch = await view();
    expect(screen.getByLabelText('status')).toHaveValue('active');
    expect(screen.getByLabelText('area')).toHaveValue('');
    expect(screen.getByText('start 2026-11-01')).toBeInTheDocument();
    expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent)).toEqual(['Plan', 'Build']);
    expect(group('no section').getByRole('listitem')).toHaveTextContent('task 2');
    expect(group('Plan').getByRole('listitem')).toHaveTextContent('task 3');
    expect(group('Build').getByRole('listitem')).toHaveTextContent('task 1');
    const lists = fetch.mock.calls.map(([url]) => url).filter((u) => u.startsWith('/api/lists'));
    expect(lists[0]).toMatch(/^\/api\/lists\/all\?today=\d{4}-\d\d-\d\d&project=2&limit=1000$/);
  });

  it('saves status and area', async () => {
    const patches: unknown[] = [];
    await view({
      'PATCH /api/projects/2': (_, init) => {
        patches.push(body(init));
        return Response.json(project(body(init)));
      },
    });
    await fireEvent.change(screen.getByLabelText('status'), { target: { value: 'someday' } });
    await fireEvent.change(screen.getByLabelText('area'), { target: { value: '9' } });
    await waitFor(() => expect(patches).toEqual([{ status: 'someday' }, { area_id: 9 }]));
    await waitFor(() => expect(screen.getByLabelText('area')).toHaveValue('9'));
  });

  it('shows project not found for a 404', async () => {
    api({ 'GET /api/projects/2': () => Response.json({ error: 'project not found' }, { status: 404 }) });
    render(ProjectView, { id: 2, kind: 'project' });
    expect(await screen.findByRole('alert')).toHaveTextContent('project not found');
  });
});

describe('sections', () => {
  it('[ add section ] prompts; Enter adds it last and refocuses the button', async () => {
    let sent: unknown;
    await view({
      'POST /api/projects/2/sections': (_, init) => {
        sent = body(init);
        return Response.json(section(12, 'Ship', 2), { status: 201 });
      },
    });
    await fireEvent.click(screen.getByRole('button', { name: '[ add section ]' }));
    const input = screen.getByRole('textbox', { name: 'section title' });
    expect(input).toHaveFocus();
    await fireEvent.input(input, { target: { value: 'Ship' } });
    await fireEvent.submit(input);
    await waitFor(() => expect(screen.getAllByRole('heading', { level: 2 }).at(-1)).toHaveTextContent('Ship'));
    expect(sent).toEqual({ title: 'Ship' });
    await waitFor(() => expect(screen.getByRole('button', { name: '[ add section ]' })).toHaveFocus());
  });

  it('shows a server error and keeps the draft; Esc cancels without leaving', async () => {
    await view({
      'POST /api/projects/2/sections': () => Response.json({ error: 'title must be 1 to 200 characters' }, { status: 400 }),
    });
    await fireEvent.click(screen.getByRole('button', { name: '[ add section ]' }));
    const input = screen.getByRole('textbox', { name: 'section title' });
    await fireEvent.submit(input);
    expect(await screen.findByRole('alert')).toHaveTextContent('error: title must be 1 to 200 characters');
    const outer = vi.fn();
    window.addEventListener('keydown', outer);
    await fireEvent.keyDown(input, { key: 'Escape' });
    window.removeEventListener('keydown', outer);
    expect(outer).not.toHaveBeenCalled();
    expect(screen.queryByRole('textbox', { name: 'section title' })).not.toBeInTheDocument();
  });

  it('[ rename ] edits the title in place', async () => {
    let sent: unknown;
    await view({
      'PATCH /api/sections/10': (_, init) => {
        sent = body(init);
        sections = [section(10, 'Research', 0), sections[1]];
        return Response.json(sections[0]);
      },
    });
    await fireEvent.click(screen.getAllByRole('button', { name: '[ rename ]' })[0]);
    const input = screen.getByRole('textbox', { name: 'rename Plan' });
    expect(input).toHaveValue('Plan');
    await fireEvent.input(input, { target: { value: 'Research' } });
    await fireEvent.submit(input);
    expect(await screen.findByRole('heading', { name: 'Research' })).toBeInTheDocument();
    expect(sent).toEqual({ title: 'Research' });
  });

  it('[ up ]/[ down ] move a section; the ends are disabled', async () => {
    const sent: unknown[] = [];
    await view({
      'PATCH /api/sections/11': (_, init) => {
        sent.push(body(init));
        sections = [section(11, 'Build', 0), section(10, 'Plan', 1)];
        return Response.json(sections[0]);
      },
    });
    const ups = screen.getAllByRole('button', { name: '[ up ]' });
    const downs = screen.getAllByRole('button', { name: '[ down ]' });
    expect(ups[0]).toBeDisabled();
    expect(downs[1]).toBeDisabled();
    await fireEvent.click(ups[1]);
    await waitFor(() => expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent)).toEqual(['Build', 'Plan']));
    expect(sent).toEqual([{ position: 0 }]);
    // Build is now first, so its [ up ] is disabled and focus goes to its [ down ].
    await waitFor(() => expect(screen.getAllByRole('button', { name: '[ down ]' })[0]).toHaveFocus());
    expect(group('Build').getByRole('listitem')).toHaveTextContent('task 1');
  });

  it('[ delete ] asks with [ no ] focused; [ no ] keeps it, [ yes ] deletes and reloads tasks', async () => {
    let deleted = false;
    const fetch = await view({
      'DELETE /api/sections/10': () => {
        deleted = true;
        return new Response(null, { status: 204 });
      },
    });
    await fireEvent.click(screen.getAllByRole('button', { name: '[ delete ]' })[0]);
    expect(screen.getByText('delete Plan?')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: '[ no ]' }));
    expect(screen.queryByText('delete Plan?')).not.toBeInTheDocument();
    await waitFor(() => expect(screen.getAllByRole('button', { name: '[ delete ]' })[0]).toHaveFocus());

    await fireEvent.click(screen.getAllByRole('button', { name: '[ delete ]' })[0]);
    expect(screen.getByRole('button', { name: '[ no ]' })).toHaveFocus();
    await fireEvent.click(screen.getByRole('button', { name: '[ yes ]' }));
    await waitFor(() => expect(screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent)).toEqual(['Build']));
    expect(deleted).toBe(true);
    expect(fetch.mock.calls.filter(([url]) => url.startsWith('/api/lists'))).toHaveLength(2);
  });
});

describe('area', () => {
  it('shows the area title, its projects, and TaskList all filtered by the area', async () => {
    const fetch = stubApi({
      'GET /api/objects/9': () => Response.json(makeObj(9, { type: 'area', title: 'Work' })),
      'GET /api/projects': () => Response.json({ projects: [project()] }),
      'GET /api/lists/all': () => Response.json({ tasks: [makeRow(4)], total: 1 }),
    });
    render(ProjectView, { id: 9, kind: 'area' });
    expect(await screen.findByRole('heading', { name: 'Work' })).toBeInTheDocument();
    expect(within(screen.getByRole('list', { name: 'projects' })).getByRole('link', { name: 'Launch' })).toHaveAttribute(
      'href',
      '#/p/2',
    );
    expect(await screen.findByRole('link', { name: 'task 4' })).toHaveAttribute('href', '#/o/4');
    const called = fetch.mock.calls.map(([url]) => url);
    expect(called).toContain('/api/projects?area=9');
    expect(called.find((u) => u.startsWith('/api/lists'))).toMatch(/^\/api\/lists\/all\?today=[\d-]+&area=9&limit=100&offset=0$/);
  });

  it('shows area not found for a 404', async () => {
    stubApi({
      'GET /api/objects/9': () => Response.json({ error: 'object not found' }, { status: 404 }),
      'GET /api/projects': () => Response.json({ projects: [] }),
    });
    render(ProjectView, { id: 9, kind: 'area' });
    expect(await screen.findByRole('alert')).toHaveTextContent('area not found');
  });
});
