#!/usr/bin/env python3
"""Builds the NL parser benchmark: `cases.jsonl` (hand-written + synthetic) and `train.jsonl`
(a disjoint synthetic split with per-word labels, for learned candidates).

Deterministic: `python3 parse/bench/gen.py` rewrites both files. Expected values are computed
here, independently of any parser. Synthetic slices: `standard` (a few known names), `cold`
(no known projects or areas) and `long` (50 projects, 50 areas, 100 tags).
"""

import calendar
import datetime as dt
import json
import os
import random
import sys

sys.dont_write_bytecode = True
import hand  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
WEEKDAYS = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"]
CODES = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"]
LONG_DAY = {1: "tues", 2: "weds", 3: "thurs"}
MONTHS = [calendar.month_name[i].lower() for i in range(1, 13)]
SMALL = (["Home Reno", "Polykov", "Garden"], ["Health", "Side Work", "Family"])
STANDARD = (["Home Reno", "Polykov", "Garden", "Q4 Launch", "Reading List"],
            ["Health", "Side Work", "Family", "Finance"])
WORDS = ["alpha", "atlas", "beacon", "cedar", "delta", "ember", "falcon", "garnet", "harbor",
         "iris", "juniper", "kestrel", "lumen", "maple", "nimbus", "orchid", "pebble", "quartz",
         "raven", "sierra"]
LONG_PROJECTS = [f"{a.title()} {b.title()}" for a, b in zip(WORDS, WORDS[5:] + WORDS[:5])] + \
    [f"{a.title()} {n}" for a in WORDS[:15] for n in ("Plan", "Ops")][:30]
LONG_AREAS = [f"{a.title()} Desk" for a in WORDS] + [f"{a.title()} Team" for a in WORDS] + \
    [f"{a.title()} Club" for a in WORDS[:10]]
LONG_TAGS = WORDS + [f"{w}/{s}" for w in WORDS for s in ("a", "b", "c", "d")]
TAGS = ["errands", "calls", "home", "work", "urgent", "family", "reading", "money", "health",
        "work/clienta", "work/clientb", "admin"]
TITLES = ["call the bank", "renew library card", "fix the gate", "email the landlord",
          "plan the offsite", "write weekly notes", "buy printer ink", "book a vet visit",
          "order new filters", "clean the oven", "draft the proposal", "review pull requests",
          "back up laptop", "pay water bill", "sort the garage", "update the resume",
          "prepare slides", "check tyre pressure", "return the parcel", "read chapter four",
          "repot the fern", "replace light bulb", "send thank you note", "test the backups",
          "schedule eye exam", "wash the car", "file receipts", "practice guitar",
          "tidy the inbox", "measure the window", "refill prescription", "call the plumber"]
# Titles with words that look like tokens but aren't (expected: no token from them).
TRAPS = ["buy sun lotion", "cancel daily newsletter", "read 1984 again", "go by train",
         "meet at the station", "watch friday night film", "update monthly report",
         "print weekly menu", "buy 2 lamps", "order 3m hooks", "prep sat practice test",
         "read the sunday paper", "pick may flowers", "fix bug #2", "email p5 vendor",
         "stack 10 boxes", "plant march seeds", "review yearly goals", "join the wed choir",
         "call daily standup host", "go in 5 minutes early", "book the mon cheri cafe"]


def nxt(d, wd):
    return d + dt.timedelta(days=(wd - d.weekday()) % 7)


def add_months(d, n):
    m = d.month - 1 + n
    y, m = d.year + m // 12, m % 12 + 1
    return dt.date(y, m, min(d.day, calendar.monthrange(y, m)[1]))


def stamp(d, t=None):
    return d.isoformat() + (f"T{t[0]:02}:{t[1]:02}" if t else "")


def cased(rng, w):
    return rng.choice([w, w, w, w.title(), w.upper()])


def date_phrase(rng, today, loose, extra=False):
    """A date phrase and its date. `loose` allows forms outside the baseline syntax; `extra`
    adds the task 051 forms (training split only, so the benchmark's synthetic cases stay
    comparable across runs)."""
    if extra and rng.random() < 0.04:
        return cased(rng, "yesterday"), today - dt.timedelta(days=1)
    k = rng.randrange(9)
    if k == 0:
        return rng.choice(["today", "tod"]), today
    if k == 1:
        return rng.choice(["tomorrow", "tmrw", "tmr"]), today + dt.timedelta(days=1)
    if k in (2, 3):
        wd = rng.randrange(7)
        if loose and wd in LONG_DAY:
            w = LONG_DAY[wd]
        else:
            w = rng.choice([WEEKDAYS[wd], WEEKDAYS[wd][:3]])
        return cased(rng, w), nxt(today, wd)
    if k == 4:
        if rng.random() < 0.5:
            return "next week", nxt(today + dt.timedelta(days=1), 0)
        return "next month", add_months(today.replace(day=1), 1)
    if k == 5:
        n, unit = rng.randint(1, 12), rng.choice(["day", "week", "month"])
        d = add_months(today, n) if unit == "month" else today + dt.timedelta(
            days=n * (7 if unit == "week" else 1))
        return f"in {n} {unit}{'s' if n > 1 or rng.random() < 0.3 else ''}", d
    if k in (6, 7):
        m = rng.randrange(12)
        day = rng.randint(1, calendar.monthrange(2027, m + 1)[1])
        d = dt.date(today.year, m + 1, day)
        if d < today:
            d = dt.date(today.year + 1, m + 1, day)
        name = rng.choice([MONTHS[m][:3], MONTHS[m]])
        suf = {1: "st", 2: "nd", 3: "rd", 21: "st", 22: "nd", 23: "rd", 31: "st"}.get(day, "th")
        return rng.choice([f"{name} {day}", f"{day} {name}", f"{name} {day}{suf}"]), d
    d = today + dt.timedelta(days=rng.randint(1, 200))
    return d.isoformat(), d


def time_phrase(rng, loose):
    if loose and rng.random() < 0.3:
        if rng.random() < 0.4:
            return "noon", (12, 0)
        h = rng.randint(1, 11)
        return f"{h} pm", (h + 12, 0)
    h, m = rng.randint(6, 21), rng.choice([0, 0, 0, 15, 30, 45])
    h12 = f"{(h - 1) % 12 + 1}{':%02d' % m if m else ''}{'pm' if h >= 12 else 'am'}"
    forms = [h12, f"at {h12}", f"{h}:{m:02}"]
    if m == 0:
        forms.append(f"at {h}")
    return rng.choice(forms), (h, m)


def duration_phrase(rng, loose):
    if loose and rng.random() < 0.3:
        n = rng.choice([1, 2, 3])
        if rng.random() < 0.5:
            return f"{n} hr{'s' if n > 1 else ''}", n * 60
        n = rng.choice([10, 15, 20, 30, 45, 90])
        return f"{n} {rng.choice(['min', 'mins'])}", n
    k = rng.randrange(5)
    if k == 0:
        n = rng.choice([5, 10, 15, 20, 25, 30, 40, 45])
        return f"{n}m", n
    if k == 1:
        n = rng.choice([15, 30, 45, 50])
        return f"{n}min", n
    if k == 2:
        n = rng.randint(1, 6)
        return f"{n}h", n * 60
    if k == 3:
        h, m = rng.randint(1, 3), rng.choice([15, 30, 45])
        return f"{h}h{m}m", h * 60 + m
    return rng.choice([("1.5h", 90), ("2.5h", 150), ("0.5h", 30)])


def day_list(rng, days):
    """Weekday names for `days`: `mon`, `mon and thu`, `mon, thu`, `mon, wed and fri`."""
    names = [rng.choice([WEEKDAYS[d], WEEKDAYS[d][:3]]) for d in days]
    if len(names) == 1:
        return names[0]
    sep = rng.choice([" and ", ", "])
    return ", ".join(names[:-1]) + sep + names[-1]


def weeks_on(rng, today, every, frm):
    """`every week|other week|N weeks on <weekday list>` (task 051)."""
    days = sorted(rng.sample(range(7), rng.choice([1, 1, 2, 3])))
    n = rng.choice([1, 2, 2, rng.randint(3, 6)])
    unit = "week" if n == 1 else rng.choice(["other week", f"{n} weeks"]) if n == 2 else f"{n} weeks"
    rule = "FREQ=WEEKLY" + (f";INTERVAL={n}" if n > 1 else "") + ";BYDAY=" + \
        ",".join(CODES[d] for d in days)
    first = min(nxt(today, d) for d in days)
    return f"{every} {unit} on {day_list(rng, days)}", rule, frm, first


def recur_phrase(rng, today, loose, extra=False):
    """A recurrence phrase, its rule, anchor and first occurrence."""
    bang = rng.random() < 0.15
    every = "every!" if bang else "every"
    frm = "completion" if bang else "scheduled"
    if extra and rng.random() < 0.15:
        return weeks_on(rng, today, every, frm)
    k = rng.randrange(7)
    if loose and rng.random() < 0.15:
        return "every other week", "FREQ=WEEKLY;INTERVAL=2", "scheduled", today
    if k == 0:
        u = rng.choice(["day", "week", "month", "year"])
        return f"{every} {u}", f"FREQ={'DAILY WEEKLY MONTHLY YEARLY'.split()[['day', 'week', 'month', 'year'].index(u)]}", frm, today
    if k == 1:
        w = rng.choice(["daily", "weekly", "monthly", "yearly"])
        return w, f"FREQ={w.upper()}", "scheduled", today
    if k == 2:
        days = sorted(rng.sample(range(7), rng.choice([1, 1, 2, 3])))
        names = [rng.choice([WEEKDAYS[d], WEEKDAYS[d][:3]]) for d in days]
        text = names[0] if len(names) == 1 else (
            " and ".join(names) if len(names) == 2 else f"{names[0]}, {names[1]} and {names[2]}")
        first = min(nxt(today, d) for d in days)
        return f"{every} {text}", "FREQ=WEEKLY;BYDAY=" + ",".join(CODES[d] for d in days), frm, first
    if k == 3:
        n, u = rng.randint(2, 12), rng.choice(["days", "weeks", "months", "years"])
        f = {"days": "DAILY", "weeks": "WEEKLY", "months": "MONTHLY", "years": "YEARLY"}[u]
        return f"{every} {n} {u}", f"FREQ={f};INTERVAL={n}", frm, today
    if k == 4:
        d = rng.choice([1, 5, 10, 15, 20, 28])
        suf = {1: "st"}.get(d, "th")
        first = today.replace(day=d) if today.day <= d else add_months(today.replace(day=1), 1).replace(day=d)
        return f"{every} {d}{suf}", f"FREQ=MONTHLY;BYMONTHDAY={d}", frm, first
    if k == 5:
        return f"{every} weekday", "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR", frm, \
            min(nxt(today, d) for d in range(5))
    return f"{every} day", "FREQ=DAILY", frm, today


def mark(rng, name):
    """How a known name is typed after its sigil: `-` or `_` for spaces, any case."""
    return rng.choice([name.lower().replace(" ", "-"), name.lower().replace(" ", "_"),
                       name.replace(" ", "-")])


def synth(rng, slice_, today, titles, traps, extra):
    """One synthetic case: (input, labels per word, expected)."""
    projects, areas, tags = {
        "standard": STANDARD + (TAGS,), "cold": ([], [], TAGS),
        "long": (LONG_PROJECTS, LONG_AREAS, LONG_TAGS)}[slice_]
    loose = rng.random() < 0.15
    title = rng.choice(traps if rng.random() < 0.12 else titles)
    exp = dict(title=title)
    parts = []  # (text, kind)
    plain = rng.random() < 0.12
    fields = [] if plain else rng.sample(
        ["date", "time", "duration", "priority", "tag", "project", "area", "deadline", "recur"],
        rng.choice([1, 2, 2, 3, 3, 4, 5]))
    if "date" in fields and "recur" in fields and rng.random() < 0.5:
        fields.remove("recur")
    date = time = first = None
    for f in fields:
        if f == "date":
            text, date = date_phrase(rng, today, loose, extra)
            parts.append((text, "date"))
        elif f == "time":
            text, time = time_phrase(rng, loose)
            parts.append((text, "time"))
        elif f == "duration":
            text, exp["estimate_min"] = duration_phrase(rng, loose)
            parts.append((text, "duration"))
        elif f == "priority":
            p = rng.randint(1, 4)
            exp["priority"] = p
            parts.append((rng.choice([f"p{p}", f"P{p}"]), "priority"))
        elif f == "tag":
            chosen = rng.sample(tags, rng.choice([1, 1, 2]))
            parts += [("#" + cased(rng, t), "tag") for t in chosen]
        elif f in ("project", "area"):
            known = projects if f == "project" else areas
            pool = [("Errands Run" if f == "project" else "Outdoors"), "Side Quest"]
            name = rng.choice(known) if known and rng.random() < 0.85 else rng.choice(pool)
            if name in known:
                exp[f] = name
            parts.append((("+" if f == "project" else "@") + mark(rng, name), f))
        elif f == "deadline":
            text, d = date_phrase(rng, today, False, extra)
            t = None
            if rng.random() < 0.3:
                tt, t = time_phrase(rng, False)
                text += " " + tt
            exp["deadline"] = stamp(d, t)
            parts.append((rng.choice(["due", "by"]) + " " + text, "deadline"))
        else:
            text, r, frm, first = recur_phrase(rng, today, loose, extra)
            exp["recurrence"] = {"rule": r, "from": frm}
            parts.append((text, "recurrence"))
    day = date or first or (today if time else None)
    if day:
        exp["scheduled"] = stamp(day, time)
    order = parts[:]
    rng.shuffle(order)
    order = fix_adjacent(order)
    if extra and rng.random() < 0.5:
        order = time_last(order)
    front = [] if not order or rng.random() < 0.75 else [order.pop(0)]
    seq = front + [(title, None)] + order
    exp["tags"] = [text[1:].lower() for text, kind in seq if kind == "tag"]
    words, labels = [], []
    for text, kind in seq:
        for i, w in enumerate(text.split()):
            words.append(w)
            labels.append("O" if kind is None else ("B-" if i == 0 else "I-") + kind)
    return " ".join(words), labels, exp


def fix_adjacent(order):
    """Keeps phrases from merging: a time never directly follows a deadline, and nothing
    follows a recurrence (a weekday list would absorb a following weekday)."""
    rec = [p for p in order if p[1] == "recurrence"]
    rest = [p for p in order if p[1] != "recurrence"]
    for i in range(1, len(rest)):
        if rest[i][1] == "time" and rest[i - 1][1] == "deadline":
            rest[i - 1], rest[i] = rest[i], rest[i - 1]
    return rest + rec


def time_last(order):
    """Moves a time after a closing recurrence (`every 2 weeks on tue 9am`): a time can't
    join a weekday list."""
    times = [p for p in order if p[1] == "time"]
    if not times or order[-1][1] != "recurrence":
        return order
    return [p for p in order if p is not times[0]] + times[:1]


def case(input_, today, slice_, projects, areas, exp):
    full = dict(title="", scheduled=None, deadline=None, estimate_min=None, priority=None,
                recurrence=None, project=None, area=None, tags=[])
    full.update(exp)
    return dict(slice=slice_, input=input_, today=str(today), projects=projects, areas=areas,
                expected=full)


def build(seed, counts, titles, traps, skip=(), extra=False):
    rng = random.Random(seed)
    out = []
    seen = set(skip)
    for slice_, n in counts:
        projects, areas = {"standard": STANDARD, "cold": ([], []),
                           "long": (LONG_PROJECTS, LONG_AREAS)}[slice_]
        while sum(1 for c in out if c[0]["slice"] == slice_) < n:
            today = dt.date(2026, 1, 1) + dt.timedelta(days=rng.randrange(730))
            text, labels, exp = synth(rng, slice_, today, titles, traps, extra)
            if text in seen:
                continue
            seen.add(text)
            out.append((case(text, today, slice_, projects, areas, exp), labels))
    return out


def main():
    ctxs = {"small": SMALL, "cold": ([], [])}
    cases = [case(i, t, "casual", *ctxs[c], e) for i, t, c, e in hand.CASES]
    bench = build(30, [("standard", 120), ("cold", 60), ("long", 60)],
                  TITLES[0::2], TRAPS[0::2])
    cases += [c for c, _ in bench]
    train = build(31, [("standard", 1500), ("cold", 500), ("long", 500)],
                  TITLES[1::2], TRAPS[1::2], skip={c["input"] for c in cases}, extra=True)
    with open(os.path.join(HERE, "cases.jsonl"), "w") as f:
        for c in cases:
            f.write(json.dumps(c, ensure_ascii=False) + "\n")
    with open(os.path.join(HERE, "train.jsonl"), "w") as f:
        for c, labels in train:
            f.write(json.dumps(dict(c, labels=labels), ensure_ascii=False) + "\n")
    print(f"cases.jsonl: {len(cases)} cases, train.jsonl: {len(train)} cases")


if __name__ == "__main__":
    main()
