import { describe, expect, it } from 'vitest';
import { chips, dayLabel, minutes, quote, segments } from './highlight';
import type { Parsed, Token } from '../lib/smartadd';

const tok = (start: number, end: number, kind: Token['kind'], text: string, resolved = true): Token => ({
  start,
  end,
  kind,
  text,
  resolved,
});

const noTask: Parsed['task'] = {
  scheduled: null,
  deadline: null,
  estimate: null,
  priority: null,
  recurrence: null,
  project_id: null,
  area_id: null,
};

describe('segments', () => {
  it('splits text into plain runs and token runs', () => {
    expect(segments('call mom p2 #fam', [tok(9, 11, 'priority', 'p2'), tok(12, 16, 'tag', '#fam')])).toEqual([
      { text: 'call mom ', kind: null, resolved: true },
      { text: 'p2', kind: 'priority', resolved: true },
      { text: ' ', kind: null, resolved: true },
      { text: '#fam', kind: 'tag', resolved: true },
    ]);
  });

  it('uses UTF-16 offsets, so an emoji before a token keeps it aligned', () => {
    const text = '🎉 party #fun +nope';
    const out = segments(text, [tok(9, 13, 'tag', '#fun'), tok(14, 19, 'project', '+nope', false)]);
    expect(out).toEqual([
      { text: '🎉 party ', kind: null, resolved: true },
      { text: '#fun', kind: 'tag', resolved: true },
      { text: ' ', kind: null, resolved: true },
      { text: '+nope', kind: 'project', resolved: false },
    ]);
    expect(out.map((s) => s.text).join('')).toBe(text);
  });

  it('skips spans that overlap or run past the text', () => {
    const out = segments('ab cd', [tok(0, 2, 'tag', 'ab'), tok(1, 3, 'tag', 'b '), tok(3, 9, 'date', 'cd…')]);
    expect(out).toEqual([
      { text: 'ab', kind: 'tag', resolved: true },
      { text: ' cd', kind: null, resolved: true },
    ]);
  });

  it('gives no segments for empty text', () => {
    expect(segments('', [])).toEqual([]);
  });
});

describe('quote', () => {
  it('wraps the token span in quotes', () => {
    expect(quote('call mom p2 now', tok(9, 11, 'priority', 'p2'))).toBe('call mom "p2" now');
  });

  it('wraps a span after an emoji by UTF-16 offset', () => {
    expect(quote('🎉 at #fun', tok(6, 10, 'tag', '#fun'))).toBe('🎉 at "#fun"');
  });
});

describe('chips', () => {
  it('labels each token with the parsed value', () => {
    const parsed: Parsed = {
      title: 'call mom',
      tags: ['family'],
      tokens: [
        tok(9, 13, 'date', 'tmrw'),
        tok(14, 17, 'time', '5pm'),
        tok(18, 21, 'duration', '90m'),
        tok(22, 24, 'priority', 'P2'),
        tok(25, 32, 'tag', '#Family'),
        tok(33, 45, 'recurrence', 'every Sunday'),
        tok(46, 53, 'deadline', 'due fri'),
        tok(54, 59, 'project', '+nope', false),
        tok(60, 63, 'date', 'tod'),
      ],
      task: { ...noTask, scheduled: '2026-10-31T17:00', deadline: '2026-11-06', estimate: 90, priority: 2 },
    };
    expect(chips(parsed, '2026-10-30').map((c) => c.label)).toEqual([
      'tomorrow',
      '17:00',
      '1h30m',
      'p2',
      '#family',
      'every sunday',
      'due 2026-11-06',
      '+nope',
      'tod',
    ]);
  });
});

describe('labels', () => {
  it('dayLabel names today and tomorrow, across a month end', () => {
    expect(dayLabel('2026-10-31', '2026-10-31')).toBe('today');
    expect(dayLabel('2026-11-01', '2026-10-31')).toBe('tomorrow');
    expect(dayLabel('2026-11-02', '2026-10-31')).toBe('2026-11-02');
  });

  it('minutes formats estimates', () => {
    expect([minutes(45), minutes(120), minutes(90)]).toEqual(['45m', '2h', '1h30m']);
  });
});
