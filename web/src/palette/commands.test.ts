import { afterEach, describe, expect, it, vi } from 'vitest';
import { announceOverlay, CAPTURE_OPEN, COMMANDS, filterCommands, OVERLAY_OPEN, subsequence } from './commands';

const labels = (query: string) => filterCommands(query).map((c) => c.label);

afterEach(() => {
  location.hash = '';
});

describe('commands', () => {
  it('lists the go-to commands and new task', () => {
    expect(COMMANDS.map((c) => c.label)).toEqual([
      'go to inbox',
      'go to today',
      'go to upcoming',
      'go to someday',
      'go to anytime',
      'go to objects',
      'go to trash',
      'go to views',
      'new task',
    ]);
  });

  it('go-to commands set the hash', () => {
    const run = (label: string) => COMMANDS.find((c) => c.label === label)!.run();
    run('go to today');
    expect(location.hash).toBe('#/today');
    run('go to trash');
    expect(location.hash).toBe('#/trash');
    run('go to views');
    expect(location.hash).toBe('#/views');
    run('go to objects');
    expect(location.hash).toBe('#/');
  });

  it('new task dispatches the capture-open event', () => {
    const opened = vi.fn();
    window.addEventListener(CAPTURE_OPEN, opened);
    COMMANDS.find((c) => c.label === 'new task')!.run();
    expect(opened).toHaveBeenCalledTimes(1);
    window.removeEventListener(CAPTURE_OPEN, opened);
  });

  it('announceOverlay dispatches overlay-open with the opening overlay', () => {
    const seen = vi.fn((e: Event) => (e as CustomEvent).detail);
    window.addEventListener(OVERLAY_OPEN, seen);
    announceOverlay('capture');
    expect(seen.mock.results.map((r) => r.value)).toEqual([{ name: 'capture' }]);
    window.removeEventListener(OVERLAY_OPEN, seen);
  });
});

describe('subsequence filter', () => {
  it('matches characters in order, ignoring case', () => {
    expect(subsequence('TDY', 'go to today')).toBe(true);
    expect(subsequence('gtt', 'go to trash')).toBe(true);
    expect(subsequence('yadot', 'go to today')).toBe(false);
    expect(subsequence('', 'anything')).toBe(true);
  });

  it('shows every command for an empty input', () => {
    expect(labels('')).toHaveLength(COMMANDS.length);
  });

  it('filters the list in order', () => {
    expect(labels('today')).toEqual(['go to today', 'go to someday']);
    expect(labels('Upc')).toEqual(['go to upcoming']);
    expect(labels('new')).toEqual(['new task']);
    expect(labels('to t')).toEqual(['go to today', 'go to anytime', 'go to objects', 'go to trash']);
    expect(labels('gos')).toEqual(['go to someday', 'go to objects', 'go to trash', 'go to views']);
    expect(labels('gtr')).toEqual(['go to trash']);
    expect(labels('zzz')).toEqual([]);
  });
});
