<script lang="ts">
  import {
    FREQS,
    WEEKDAYS,
    buildRule,
    describeRecurrence,
    parseRule,
    type Freq,
    type Recurrence,
    type RuleParts,
    type Weekday,
  } from '../lib/tasks';

  // The task's recurrence editor. Every control change calls `onsave` once with the new value
  // (`null` for none); the parent sends it and shows a server error.
  let { value, onsave }: { value: Recurrence | null; onsave: (r: Recurrence | null) => void } = $props();

  const parts = $derived<RuleParts | null>(value ? parseRule(value.rule) : null);
  const from = $derived(value?.from ?? 'scheduled');

  function save(next: Partial<RuleParts>, anchor = from) {
    const base: RuleParts = parts ?? { freq: 'day', interval: 1, byDay: [], monthDay: null };
    onsave({ rule: buildRule({ ...base, ...next }), from: anchor });
  }

  function setFreq(f: string) {
    if (f === '') onsave(null);
    else save({ freq: f as Freq });
  }

  function toggleDay(d: Weekday) {
    const days = parts?.byDay ?? [];
    save({ byDay: days.includes(d) ? days.filter((x) => x !== d) : [...days, d] });
  }

  /** A number input's value, or `fallback` when it is empty. */
  function num(input: HTMLInputElement, fallback: number | null): number | null {
    return input.value === '' ? fallback : Number(input.value);
  }
</script>

<div class="recurrence">
  <p class="text">{value ? describeRecurrence(value) : 'does not repeat'}</p>
  <div class="row">
    <select aria-label="repeat" value={parts?.freq ?? ''} onchange={(e) => setFreq(e.currentTarget.value)}>
      <option value="">none</option>
      {#each FREQS as f (f)}<option value={f}>{f}</option>{/each}
    </select>
    {#if parts}
      <label>
        every
        <input
          type="number"
          min="1"
          max="365"
          aria-label="interval"
          value={parts.interval}
          onchange={(e) => save({ interval: num(e.currentTarget, 1) ?? 1 })}
        />
      </label>
      {#if parts.freq === 'month'}
        <label>
          day
          <input
            type="number"
            min="1"
            max="31"
            aria-label="month day"
            value={parts.monthDay ?? ''}
            onchange={(e) => save({ monthDay: num(e.currentTarget, null) })}
          />
        </label>
      {/if}
      <select aria-label="repeat from" value={from} onchange={(e) => save({}, e.currentTarget.value as Recurrence['from'])}>
        <option value="scheduled">from scheduled</option>
        <option value="completion">from completion</option>
      </select>
    {/if}
  </div>
  {#if parts?.freq === 'week'}
    <div class="days" role="group" aria-label="weekdays">
      {#each WEEKDAYS as d (d)}
        <button type="button" aria-pressed={parts.byDay.includes(d)} onclick={() => toggleDay(d)}>{d}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .recurrence {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .text {
    margin: 0;
    color: var(--dim);
  }
  .row,
  .days {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 1ch;
  }
  label {
    display: flex;
    align-items: center;
    gap: 0.5ch;
    color: var(--dim);
  }
  select,
  input,
  button {
    min-height: 44px;
    max-width: 100%;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
  }
  input {
    width: 7ch;
    min-width: 44px;
    border-bottom: 1px solid var(--line);
  }
  input:focus {
    outline: none;
    border-bottom-color: var(--accent);
  }
  button {
    min-width: 44px;
    padding: 0;
    color: var(--dim);
    cursor: pointer;
  }
  button[aria-pressed='true'] {
    color: var(--accent);
    text-decoration: underline;
  }
</style>
