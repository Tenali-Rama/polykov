// Project and section calls (server API "Projects"), all built on `send`.
import { send } from './api';
import type { Obj } from './objects';

export const PROJECT_STATUSES = ['active', 'someday', 'done'] as const;
export type ProjectStatus = (typeof PROJECT_STATUSES)[number];

export type ProjectFields = {
  status: ProjectStatus;
  area_id: number | null;
  start: string | null;
  deadline: string | null;
  open_tasks: number;
  done_tasks: number;
};
/** The project JSON: the object JSON (type `project`) plus its project fields. */
export type Project = Obj & { project: ProjectFields };
/** `null` clears a field. */
export type ProjectChanges = Partial<Pick<ProjectFields, 'status' | 'area_id' | 'start' | 'deadline'>>;
export type Section = { id: number; project_id: number; title: string; position: number };

/** Live, non-archived projects by title; the server's default status is `active`. */
export async function listProjects(filter: { status?: ProjectStatus | 'any'; area?: number } = {}): Promise<Project[]> {
  const q = new URLSearchParams();
  if (filter.status) q.set('status', filter.status);
  if (filter.area !== undefined) q.set('area', String(filter.area));
  const query = q.size > 0 ? `?${q}` : '';
  const { projects } = (await (await send('GET', `/api/projects${query}`)).json()) as { projects: Project[] };
  return projects;
}

/** Throws `ApiError` 404 (`project not found`). */
export async function getProject(id: number): Promise<Project> {
  return (await send('GET', `/api/projects/${id}`)).json();
}

export async function updateProject(id: number, changes: ProjectChanges): Promise<Project> {
  return (await send('PATCH', `/api/projects/${id}`, changes)).json();
}

/** By position. */
export async function listSections(project: number): Promise<Section[]> {
  const { sections } = (await (await send('GET', `/api/projects/${project}/sections`)).json()) as {
    sections: Section[];
  };
  return sections;
}

/** Adds a section, last unless `position` is given. */
export async function createSection(project: number, title: string, position?: number): Promise<Section> {
  return (await send('POST', `/api/projects/${project}/sections`, { title, position })).json();
}

/** Renames and/or moves a section. */
export async function updateSection(id: number, changes: { title?: string; position?: number }): Promise<Section> {
  return (await send('PATCH', `/api/sections/${id}`, changes)).json();
}

/** Its tasks stay in the project, unsectioned. */
export async function deleteSection(id: number): Promise<void> {
  await send('DELETE', `/api/sections/${id}`);
}
