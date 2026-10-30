import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { stubApi } from '../objects/testing';
import type { Recurrence, Task } from '../lib/tasks';
import TaskFields from './TaskFields.svelte';
import { makeRow } from './testing';

const SAVED: Recurrence = { rule: 'FREQ=WEEKLY;INTERVAL=2;BYDAY=MO', from: 'scheduled' };

/** Task 7 recurring per `SAVED`; a PATCH whose rule `reject` matches gets a 400, any other is saved. */
async function open(reject: RegExp) {
  const row = makeRow(7, { recurrence: SAVED });
  let current: Task = { ...row, subtasks: undefined } as unknown as Task;
  const patches: string[] = [];
  stubApi({
    'GET /api/tasks/7': () => Response.json(current),
    'GET /api/projects': () => Response.json({ projects: [] }),
    'GET /api/objects': () => Response.json({ objects: [], total: 0 }),
    'PATCH /api/tasks/7': (_url, init) => {
      const { task } = JSON.parse(String(init?.body)) as { task: { recurrence: Recurrence } };
      patches.push(task.recurrence.rule);
      if (reject.test(task.recurrence.rule)) return Response.json({ error: 'invalid recurrence rule' }, { status: 400 });
      current = { ...current, task: { ...current.task, ...task } };
      return Response.json(current);
    },
  });
  render(TaskFields, { props: { id: 7 } });
  await screen.findByText('every 2 weeks on mon · from scheduled');
  return patches;
}

const field = (name: string) => screen.getByLabelText(name) as HTMLInputElement;
const day = (name: string) => screen.getByRole('button', { name });

afterEach(() => vi.unstubAllGlobals());

describe('recurrence editor', () => {
  it('a rejected interval shows the error and puts the saved interval back', async () => {
    const patches = await open(/INTERVAL=0/);
    await fireEvent.change(field('interval'), { target: { value: '0' } });
    expect(await screen.findByRole('alert')).toHaveTextContent('invalid recurrence rule');
    expect(patches).toEqual(['FREQ=WEEKLY;INTERVAL=0;BYDAY=MO']);
    await waitFor(() => expect(field('interval')).toHaveValue(2));
    expect(field('repeat')).toHaveValue('week');
    expect(day('mon')).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText('every 2 weeks on mon · from scheduled')).toBeInTheDocument();
  });

  it('a rejected frequency puts the saved one back', async () => {
    await open(/MONTHLY/);
    await fireEvent.change(field('repeat'), { target: { value: 'month' } });
    await screen.findByRole('alert');
    await waitFor(() => expect(field('repeat')).toHaveValue('week'));
    expect(screen.queryByLabelText('month day')).toBeNull();
  });

  it('a valid change sticks and clears the error', async () => {
    const patches = await open(/INTERVAL=0/);
    await fireEvent.change(field('interval'), { target: { value: '0' } });
    await screen.findByRole('alert');
    await fireEvent.change(field('interval'), { target: { value: '3' } });
    await screen.findByText('every 3 weeks on mon · from scheduled');
    expect(field('interval')).toHaveValue(3);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(patches).toEqual(['FREQ=WEEKLY;INTERVAL=0;BYDAY=MO', 'FREQ=WEEKLY;INTERVAL=3;BYDAY=MO']);
  });

  it('a rejected weekday toggle leaves the saved weekdays pressed', async () => {
    const patches = await open(/TU/);
    await fireEvent.click(day('tue'));
    expect(await screen.findByRole('alert')).toHaveTextContent('invalid recurrence rule');
    expect(patches).toEqual(['FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TU']);
    expect(day('tue')).toHaveAttribute('aria-pressed', 'false');
    expect(day('mon')).toHaveAttribute('aria-pressed', 'true');
    // The next toggle builds on the saved rule, not the rejected one.
    await fireEvent.click(day('wed'));
    await waitFor(() => expect(day('wed')).toHaveAttribute('aria-pressed', 'true'));
    expect(patches[1]).toBe('FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE');
  });
});
