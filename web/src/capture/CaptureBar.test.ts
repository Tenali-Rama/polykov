import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import CaptureBar from './CaptureBar.svelte';
import type { Parsed, Token } from '../lib/smartadd';
import { CAPTURE_OPEN, PALETTE_OPEN } from '../palette/commands';

type Call = { url: string; body: { text: string; today: string }; signal?: AbortSignal | null };

const noTask: Parsed['task'] = {
  scheduled: null,
  deadline: null,
  estimate: null,
  priority: null,
  recurrence: null,
  project_id: null,
  area_id: null,
};

/** A parse of `text` with a `p2` token wherever `p2` occurs (unquoted). */
function parsedFor(text: string): Parsed {
  const tokens: Token[] = [];
  const re = /(?<!")\bp2\b(?!")/g;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    tokens.push({ start: m.index, end: m.index + 2, kind: 'priority', text: 'p2', resolved: true });
  }
  return { title: text, tokens, tags: [], task: { ...noTask, priority: tokens.length ? 2 : null } };
}

/** Stub fetch: `/api/parse` answers via `parseReply` (default: at once), `/api/tasks/add` via `addReply`. */
function stub(
  opts: {
    parseReply?: (call: Call) => Promise<Response>;
    addReply?: (call: Call) => Response;
  } = {},
) {
  const calls: Call[] = [];
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    const call = { url, body: JSON.parse(String(init?.body)), signal: init?.signal };
    calls.push(call);
    if (url === '/api/parse') return opts.parseReply ? opts.parseReply(call) : Response.json(parsedFor(call.body.text));
    if (url === '/api/tasks/add') {
      const reply = opts.addReply ?? (() => Response.json(added(call.body.text), { status: 201 }));
      return reply(call);
    }
    return new Response(null, { status: 599 });
  });
  vi.stubGlobal('fetch', fn);
  return calls;
}

function added(title: string, scheduled: string | null = null) {
  return {
    id: 1,
    type: 'task',
    title,
    tags: [],
    task: { scheduled, deadline: null, project_id: null, area_id: null, parent_id: null, someday: false },
  };
}

function setup() {
  document.body.innerHTML = '<input aria-label="other" />';
  const other = screen.getByRole('textbox', { name: 'other' });
  other.focus();
  render(CaptureBar);
  return other;
}

async function openBar(target: Element | Window = document.activeElement ?? window) {
  await fireEvent.keyDown(target, { key: ' ', code: 'Space', ctrlKey: true });
  const input = await screen.findByRole('textbox', { name: 'capture' });
  await waitFor(() => expect(input).toHaveFocus());
  return input;
}

const dialog = () => screen.queryByRole('dialog', { name: 'capture' });

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  document.body.innerHTML = '';
});

describe('capture bar', () => {
  it('Ctrl+Space opens it from another input; Esc closes it and restores focus', async () => {
    const calls = stub();
    const other = setup();
    expect(dialog()).toBeNull();
    const input = await openBar(other);
    expect(dialog()).toBeInTheDocument();
    await fireEvent.input(input, { target: { value: 'draft' } });
    const esc = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    input.dispatchEvent(esc);
    expect(esc.defaultPrevented).toBe(true);
    await waitFor(() => expect(dialog()).toBeNull());
    expect(other).toHaveFocus();
    expect(calls.filter((c) => c.url === '/api/tasks/add')).toEqual([]);
  });

  it('highlights tokens in the mirror and shows chips', async () => {
    stub();
    setup();
    const input = await openBar();
    await fireEvent.input(input, { target: { value: 'call mom p2' } });
    const chip = await screen.findByRole('button', { name: 'unparse p2' });
    expect(chip).toHaveTextContent('[ x ]');
    expect(within(screen.getByRole('list', { name: 'parsed' })).getByRole('listitem')).toHaveTextContent('p2');
    const tok = dialog()!.querySelector('.mirror .tok.priority');
    expect(tok).toHaveTextContent('p2');
    expect(dialog()!.querySelector('.mirror')).toHaveTextContent('call mom p2');
  });

  it('[ x ] quotes the token so it stays title text, and re-parses', async () => {
    const calls = stub();
    setup();
    const input = (await openBar()) as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'buy p2 now' } });
    await fireEvent.click(await screen.findByRole('button', { name: 'unparse p2' }));
    expect(input.value).toBe('buy "p2" now');
    await waitFor(() => expect(calls.at(-1)?.body.text).toBe('buy "p2" now'));
    await waitFor(() => expect(screen.queryByRole('button', { name: 'unparse p2' })).toBeNull());
    expect(input).toHaveFocus();
  });

  it('cancels the previous parse and ignores a stale response', async () => {
    const replies: Array<(r: Response) => void> = [];
    const calls = stub({ parseReply: () => new Promise((resolve) => replies.push(resolve)) });
    setup();
    const input = await openBar();
    await fireEvent.input(input, { target: { value: 'a p2' } });
    await fireEvent.input(input, { target: { value: 'a' } });
    expect(calls[0].signal?.aborted).toBe(true);
    expect(calls[1].signal?.aborted).toBe(false);
    replies[1](Response.json(parsedFor('a')));
    await waitFor(() => expect(calls).toHaveLength(2));
    replies[0](Response.json(parsedFor('a p2')));
    await new Promise((r) => setTimeout(r, 10));
    expect(screen.queryByRole('button', { name: 'unparse p2' })).toBeNull();
  });

  it('Enter saves, closes, restores focus, notes it for 4 s and dispatches tasks-changed', async () => {
    const calls = stub({
      addReply: () => Response.json(added('call mom', '2999-01-01T17:00'), { status: 201 }),
    });
    const other = setup();
    const changed = vi.fn();
    window.addEventListener('polykov:tasks-changed', changed);
    const input = await openBar();
    await fireEvent.input(input, { target: { value: 'call mom p2' } });
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    await fireEvent.submit(input.closest('form')!);
    await vi.waitFor(() => expect(dialog()).toBeNull());
    expect(calls.find((c) => c.url === '/api/tasks/add')?.body.text).toBe('call mom p2');
    expect(other).toHaveFocus();
    expect(screen.getByRole('status')).toHaveTextContent('added: call mom · upcoming');
    expect(changed).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(3900);
    expect(screen.queryByRole('status')).toBeInTheDocument();
    await vi.advanceTimersByTimeAsync(200);
    expect(screen.queryByRole('status')).toBeNull();
    window.removeEventListener('polykov:tasks-changed', changed);
  });

  it('notes an inbox add as added to inbox', async () => {
    stub();
    setup();
    const input = await openBar();
    await fireEvent.input(input, { target: { value: 'buy milk' } });
    await fireEvent.submit(input.closest('form')!);
    expect(await screen.findByRole('status')).toHaveTextContent('added to inbox: buy milk');
  });

  it('keeps the bar open with the error and the text on a 400', async () => {
    const changed = vi.fn();
    window.addEventListener('polykov:tasks-changed', changed);
    stub({ addReply: () => Response.json({ error: 'unknown project: nope' }, { status: 400 }) });
    setup();
    const input = (await openBar()) as HTMLInputElement;
    await fireEvent.input(input, { target: { value: 'x +nope' } });
    await fireEvent.submit(input.closest('form')!);
    expect(await screen.findByRole('alert')).toHaveTextContent('unknown project: nope');
    expect(dialog()).toBeInTheDocument();
    expect(input.value).toBe('x +nope');
    expect(changed).not.toHaveBeenCalled();
    window.removeEventListener('polykov:tasks-changed', changed);
  });

  it('does nothing on Enter with an empty input', async () => {
    const calls = stub();
    setup();
    const input = await openBar();
    await fireEvent.input(input, { target: { value: '   ' } });
    await fireEvent.submit(input.closest('form')!);
    expect(calls).toEqual([]);
    expect(dialog()).toBeInTheDocument();
  });

  it('Tab and Shift+Tab wrap within the open bar', async () => {
    stub();
    setup();
    const input = await openBar();
    await fireEvent.input(input, { target: { value: 'a p2 p2' } });
    const chips = await screen.findAllByRole('button', { name: 'unparse p2' });
    chips[1].focus();
    const tab = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true });
    chips[1].dispatchEvent(tab);
    expect(tab.defaultPrevented).toBe(true);
    expect(input).toHaveFocus();
    await fireEvent.keyDown(input, { key: 'Tab', shiftKey: true });
    expect(chips[1]).toHaveFocus();
    // A Tab between stops is left to the browser.
    chips[0].focus();
    const inner = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true });
    chips[0].dispatchEvent(inner);
    expect(inner.defaultPrevented).toBe(false);
  });

  it('shows an unresolved project chip in the error colour', async () => {
    stub({
      parseReply: async () =>
        Response.json({
          ...parsedFor('x +nope'),
          tokens: [{ start: 2, end: 7, kind: 'project', text: '+nope', resolved: false }],
        }),
    });
    setup();
    const input = await openBar();
    await fireEvent.input(input, { target: { value: 'x +nope' } });
    const chip = await screen.findByRole('button', { name: 'unparse +nope' });
    expect(chip.closest('li')).toHaveClass('unresolved');
    expect(dialog()!.querySelector('.mirror .tok.unresolved')).toHaveTextContent('+nope');
  });

  it('> typed into the empty line closes the bar and opens the palette with the rest', async () => {
    const calls = stub();
    const other = setup();
    const opened = vi.fn((e: Event) => [(e as CustomEvent).detail, document.activeElement]);
    window.addEventListener(PALETTE_OPEN, opened);
    const input = await openBar(other);
    await fireEvent.input(input, { target: { value: '>tod' } });
    await waitFor(() => expect(dialog()).toBeNull());
    expect(opened).toHaveBeenCalledTimes(1);
    // The palette sees the element the bar was opened from as the one to return to.
    expect(opened.mock.results[0].value).toEqual([{ text: 'tod' }, other]);
    expect(calls).toEqual([]);
    window.removeEventListener(PALETTE_OPEN, opened);
  });

  it('> after other text is title text, not a handoff', async () => {
    const calls = stub();
    setup();
    const opened = vi.fn();
    window.addEventListener(PALETTE_OPEN, opened);
    const input = await openBar();
    await fireEvent.input(input, { target: { value: 'a' } });
    await fireEvent.input(input, { target: { value: '>a' } });
    expect(opened).not.toHaveBeenCalled();
    expect(dialog()).toBeInTheDocument();
    await waitFor(() => expect(calls.at(-1)?.body.text).toBe('>a'));
    window.removeEventListener(PALETTE_OPEN, opened);
  });

  it('the capture-open event opens it (the palette\'s new task)', async () => {
    stub();
    setup();
    window.dispatchEvent(new CustomEvent(CAPTURE_OPEN));
    const input = await screen.findByRole('textbox', { name: 'capture' });
    await waitFor(() => expect(input).toHaveFocus());
  });
});
