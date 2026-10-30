// Test fixtures for task views.
import type { Row, TaskFields } from '../lib/tasks';
import { makeObj } from '../objects/testing';

/** A list row: an open P4 task with no dates and no sub-tasks unless `task`/`over` say otherwise. */
export function makeRow(id: number, task: Partial<TaskFields> = {}, over: Partial<Row> = {}): Row {
  return {
    ...makeObj(id, { type: 'task', title: `task ${id}` }),
    task: {
      status: 'open',
      priority: 4,
      scheduled: null,
      deadline: null,
      estimate: null,
      someday: false,
      parent_id: null,
      project_id: null,
      section_id: null,
      area_id: null,
      checklist: [],
      recurrence: null,
      completed: null,
      ...task,
    },
    subtasks: { open: 0, total: 0 },
    ...over,
  };
}
