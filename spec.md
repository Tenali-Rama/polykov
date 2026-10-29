# Polykov — Spec

All-in-one, self-hosted productivity tool: tasks, notes, calendar, habits, goals, journaling, and analytics, all linked together in one system.

## 1. Principles

- **Single user, self-hosted.** Each install serves one owner, who runs it on their own machine/VM/cloud. No multi-user, no sharing, no collaboration, no telemetry, no third-party data calls.
- **User owns the data.** Data lives only on the user's server. Exports are plain formats (MD, CSV, JSON).
- **Fast.** Rust core. UI interactions < 50 ms, search < 100 ms on 100k objects.
- **Local intelligence.** All ML/AI runs on the user's server. Models are small and CPU-friendly but high quality. Nothing goes to external APIs.
- **Everything connected.** Every entity is an "object" that can be tagged, linked, pinned, searched, filtered, and shown in any view.
- **Clean, minimal, focus-first UI.** Little on screen, lots of whitespace, no visual noise. Panels and controls show only when needed.
- **CLI look and feel** on every client: monospace, terminal look, keyboard-first, still touch-friendly on Android.
- **Modular.** Every module can be switched on or off.
- Open source on GitHub under **AGPL-3.0**. One-command install.

## 2. Architecture & Stack

- **Server (`polykov-server`)**: Rust, Axum, SQLite (WAL mode, FTS5 for full-text search), Tokio.
  - REST + WebSocket API (live updates pushed to all connected clients).
  - Background job runner: reminders, recurring tasks, Telegram polling, ML training, backups.
  - Single binary plus a Docker image.
- **ML runtime**: in-process Rust, CPU-only. Runtimes and models are chosen through experiments (see §5). Models download on first run and live in the data dir.
- **Web UI**: Svelte + TypeScript, served by the server. One codebase shared by all GUI clients.
  - Note editor: TipTap/ProseMirror (blocks, toggles, wikilinks, markdown round-trip).
- **Linux desktop + Android (phone/tablet)**: Tauri 2 wrapping the web UI.
  - Android widgets: small native Kotlin module.
  - Floating timer: a separate always-on-top window on desktop, a notification/overlay on Android.
- **CLI (`polykov`)**: Rust (`clap`), plus an optional full-screen TUI (`ratatui`). Talks to the server API.
- **Sync model**: the server is the source of truth. Clients cache recent data for fast loads and queue writes while offline. Conflicts resolve last-write-wins per field.
- **Data dir**: `polykov.db`, `attachments/`, `models/`, `backups/`, `config.toml`.

## 3. Data Model

- **Object** (base for everything): `id`, `type`, `title`, `created`, `updated`, `archived`, `pinned`, `tags[]`, `properties{}` (YAML-like custom fields), `links[]`, `comments[]`, `attachments[]`.
- **Types**:
  - **Task**: status, priority P1–P4, start/scheduled date, deadline, duration/estimate, actual time, recurrence rule, buffer time, checklist, sub-tasks (unlimited depth), section, project, area, goal/milestone.
  - **Project**: sections, tasks, notes, area, goal, status, dates.
  - **Area**: a long-running responsibility (Health, Work) that groups projects, notes, habits, and goals.
  - **Goal → Milestones**: target date, progress (auto-computed from linked tasks/milestones/habits).
  - **Habit**: schedule (daily, X per week, custom), check-in log, streaks, target.
  - **Note**: block-based markdown content, belongs to a notebook.
  - **Notebook**: nested container for notes.
  - **Journal entry**: timestamped interstitial log line, attached to a daily note. Optional mood (1–5) and energy (1–5), an auto activity label, and the linked active task/timer.
  - **Daily note**: one per date. Pulls in that day's journal entries, calendar, completed tasks, and habits.
  - **Review**: daily, weekly, monthly, quarterly, or yearly, from a template.
  - **Event**: calendar event, either internal or synced from outside.
  - **Time entry**: start, end, linked object (task, project, ...).
  - **View**: a saved search/filter plus a layout.
  - **Forecast record**: model, target object, predicted range, actual outcome, feedback reason/text.
- **Tags**: hierarchical (`work/clientA/design`), and act as folders.
- **Links**: any object to any object. Backlinks are computed automatically.
- **History**: completed and archived objects are kept and searchable. Soft delete, with a trash that empties after 30 days.

## 4. Features

### Capture
- **Global capture bar**: one shortcut, from anywhere, opens a minimal capture bar. Enter saves to the Inbox through smart add, without leaving the current screen. Typing `>` switches it to the command palette.
  - Web/app: in-app shortcut (default `Ctrl+Space`).
  - Linux: system-wide hotkey, works even when the app is in the background.
  - Android: quick-settings tile, home-screen widget, and share-to-Polykov.
- **Inbox**: the default destination for new tasks and all Telegram input, unless the user names a target.
- **Smart add**: a single input line with a natural-language parser.
  - Picks out date/time, recurrence, duration, priority, project, tags, area, and deadline (e.g. `call mom tmrw 5pm 30m p2 #family every sunday`).
  - Parsed tokens are highlighted live and can be edited before saving.
- **Voice input**: upload or record English audio; transcribed locally, then sent through smart add.
- **Import**: CSV (tasks), Markdown (notes, with front-matter → properties), with field mapping.
- **Export**: everything as MD + CSV + JSON.

### Tasks (GTD)
- **Today / Upcoming / Someday / Inbox / Anytime** views.
- Projects → sections/headings → tasks → sub-tasks → checklists.
- P1–P4 priorities, labels (tags), deadlines vs scheduled dates, recurring tasks (fixed date or "after completion").
- Estimates vs actual time spent, buffer time before/after.
- Comments and image attachments on tasks.
- Grouping, sorting, and **smart sort** (ranks by priority, deadline proximity, estimate, and goal relevance).
- **Layouts** (available for any task list or view): list, Kanban board, Eisenhower matrix, calendar, timeline/Gantt.

### Notes
- Full markdown (GFM, code blocks with highlighting, tables, math, mermaid).
- Notion-style blocks: drag to reorder, toggles, callouts, embeds of tasks/views.
- Wikilinks `[[...]]` to any object, with backlinks panel.
- YAML properties (front-matter), queryable in views.
- **Focus mode** toggle (shortcut): hides the sidebar, toolbars, panels, and everything except the note text, centered. Optional typewriter scrolling and dimming of inactive paragraphs.
- Floating table of contents.
- Image attachments (paste/drop).
- **Link previews**: toggleable hover/inline preview for internal and external links.

### Calendar & Planning
- Day, week, and month views.
- Two-way sync with external calendars over CalDAV (Google, Nextcloud, etc.), plus ICS subscriptions.
- **Time blocking**: drag tasks onto the calendar. Duration and buffer are respected, and over-booked days are flagged.
- **Day planner**: proposes a schedule for today from open tasks, estimates, deadlines, and free slots. The user accepts or edits it.
- Daily-note journal entries show up on the timeline.

### Journaling
- **Interstitial journaling**: quick timestamped entries, capturable from anywhere (CLI, Telegram, hotkey, widget).
- **Daily note**: created automatically and linked to that date's calendar, tasks, and habits.
- **Mood & energy**: optional one-tap 1–5 ratings on each entry. Skipping them never blocks capture.
- **Activity label**: the local model labels each entry automatically (deep work, admin, break, distraction, …). Labels can be edited, and corrections retrain the model.
- **Task context**: if a timer is running when an entry is written, the entry is linked to that task.
- **Journal analytics**:
  - mood/energy by time of day and weekday
  - time spent per activity label
  - how distractions relate to focus time
  - topic clustering and word frequency
  - links to productivity metrics (completions, estimate accuracy)

### Habits
- Habit library (preset templates plus custom).
- Daily check-ins, a streak counter, a calendar heatmap, and a check-in log.
- Can be linked to goals and areas.

### Goals
- Goals → milestones → linked tasks, projects, and habits.
- Progress is computed automatically, with forecasted completion.

### Time & Focus
- One-click time tracking on any task or project.
- Timer modes: stopwatch, countdown, pomodoro (configurable), and duration-based (counts down from the task's estimate).
- Floating timer window.
- Ambient focus sounds (bundled, offline: rain, café, white/brown noise), mixable.
- Tracked time feeds into actual-vs-estimate and analytics.

### Reviews
- Daily, weekly, monthly, quarterly, and yearly templates (editable).
- Each one fills itself in automatically with stats for the period: completed tasks, time per area/project, habit streaks, goal progress, and journal highlights.
- Review prompts are scheduled as reminders.

### Analytics & Forecasts
- **Rule:** only include analytics that lead to a decision. No vanity metrics; karma stays in Gamification.
- **Accuracy rules** (apply to every forecast and insight):
  - No forecast until there is enough history (e.g. 3+ weeks); until then show "learning…".
  - Forecasts show ranges with a confidence level (e.g. "12–15 Nov, 80%"), never a single number.
  - Each model is backtested against actual outcomes, and its hit rate is shown. A model that keeps failing is hidden automatically.
  - Every insight links to the data behind it.
- **Planning & workload**:
  - Workload forecast: planned hours vs real capacity per day/week, with overload warnings.
  - Personal estimate accuracy per tag/project/type (e.g. "writing: underestimated by 40%"). The day planner corrects estimates using it.
  - Completion forecast per task/project/deadline, with deadline-risk flags.
- **Time & focus**:
  - Time spent by area, project, and tag.
  - Deep-work hours and their trend.
  - Best focus windows, which the planner uses.
  - Pomodoro: completed vs abandoned sessions, and interruptions.
  - Distraction patterns, from journal activity labels.
- **Tasks**:
  - Completion rate and weekly throughput.
  - Overdue and postponed tasks: which ones keep slipping.
  - Inbox processing speed.
  - Cycle time (created → done).
- **Habits**:
  - Consistency score.
  - Streak-risk forecast for the week.
  - Best days and times per habit.
- **Goals**:
  - Progress vs ideal pace.
  - ETA forecast with a range.
  - Alerts for goals and areas with no activity for X weeks.
- **Journal & wellbeing**:
  - Mood and energy by hour and weekday.
  - Correlations with habits, activities, and workload (e.g. "energy higher on days with morning habit").
  - Topic trends.
- **Reviews**: each period gets an auto summary — wins, slips, change vs the previous period, and 2–3 concrete suggestions.
- **Forecast feedback loop**:
  - After a forecast's outcome is known, the app shows predicted vs actual and asks for optional quick feedback: what went wrong.
    - Preset reasons: scope grew, got sick/unavailable, priorities changed, bad estimate, external blocker, other.
    - Optional free text.
  - Any forecast or insight can also be marked "wrong" or "not useful" at any time.
  - The models learn from this automatically:
    - Outcomes and reasons become training signal; models retrain in the background.
    - Reasons outside the user's control (sick, external blocker) count as one-off disruptions and don't distort future forecasts.
    - Reasons like scope creep or bad estimates adjust the user's personal bias factors.
    - Insights marked "not useful" are shown less often.
  - A model accuracy page shows each model's hit rate over time, so improvement is visible.
- Every chart can be filtered by the same query language as Views.

### Organization
- **Views**: saved searches with a filter query language. Example:
  - `type:task priority:<=2 due:<7d tag:work/* -status:done`
  - Each view has its own layout, grouping, and sort, and can be pinned to the sidebar.
- **Favourites**: pin any object.
- **Archive** and **completed history**: browsable and searchable.
- **Global search**: full-text across all objects, with fuzzy matching. Command palette with `Ctrl+K`.

### Reminders & Notifications
- Per-object reminders (absolute or relative to due/start).
- Delivered via web push, desktop notifications, Android notifications, and Telegram.
- Quiet hours.

### Gamification (toggleable)
- Karma points for completions, streaks, and reviews.
- Levels, achievements, and badges in pixel-art style.
- Daily/weekly karma goals.

## 5. Intelligence (local models)

- **Models are not fixed in this spec.** For each ML component, builders test several candidate approaches, from simple baselines up to small ML models, and ship whichever wins on the benchmark. The spec defines only what each component must do and how winners are chosen.
- **Components and their targets**:
  - **NL parser**: extracts date/time, recurrence, duration, priority, project, tags, area, and deadline. Must run in < 20 ms.
  - **Auto-tagger**: suggests tags, project, area, and priority. Learns from the user's history and from accepted/rejected suggestions.
  - **Speech-to-text**: English, high accuracy on casual speech, CPU-only, faster than real-time on a 2 vCPU VM.
  - **Forecasting**: every forecast in §4 Analytics. Must output calibrated ranges, and learn from outcomes and feedback (see Forecast feedback loop).
  - **Journal analysis**: activity labels (learning from user corrections), topic clustering, and sentiment.
- **Experiment process** (for every component):
  - Build a benchmark set:
    - synthetic and anonymized sample data
    - a "cold start" case (little history)
    - a "long history" case
  - Always include a simple baseline, such as rules or historical averages. An ML model ships only if it beats the baseline.
  - Measure accuracy and calibration, latency, RAM, model size, and cold-start behaviour. Per-component metrics are defined in the benchmark (e.g. interval coverage for forecasts, F1 for the tagger, WER for speech).
  - Record each experiment's results in `docs/models/` (candidates, metrics, decision), and re-run them when new candidates appear.
- **Model interface**: every component sits behind a stable Rust trait. Models can be swapped without touching the rest of the app.
- **On-device learning**:
  - Models retrain in the background on the user's own data.
  - A new version replaces the current one only if it scores better on the user's recent history (shadow evaluation).
  - Past versions are kept for rollback.
- **Constraints**: fully local, CPU-only, total model RAM ≤ 1 GB, and no data leaves the server.
- Every ML feature can be switched off, and models can be swapped in `config.toml`.

## 6. Telegram Integration

- The user creates their own bot and enters its token. Only the owner's chat ID is accepted; everything else is ignored.
- Text and voice messages go to the Inbox through smart add. Voice is transcribed locally.
- Command prefixes: `/j` (journal entry), `/n` (note), `/t` (task with target). Defaults to inbox task.
- Commands: `/today`, `/done <id>`, `/habit <name>`, `/timer`.
- Receives reminders and daily/weekly digests.

## 7. Clients & UI

- **Web**: the full app, keyboard-first.
- **Linux app**: Tauri. Adds a global quick-capture hotkey, tray icon, floating timer, and native notifications.
- **Android (phone + tablet)**: Tauri.
  - Touch-optimized: large hit targets, swipe actions, split-pane layout on tablets.
  - Widgets: Today list, quick add, habit check-in, timer.
- **CLI**: full CRUD, smart add, views, timer, journal, and JSON output for scripting. Example: `polykov add "..."`, `polykov today`, `polykov j "..."`.
- **TUI** (`polykov tui`): interactive terminal version of the main views.
- **Look**: terminal look everywhere — monospace font, box-drawing borders, prompt-style inputs, block cursor. Still readable and fully touch-friendly.
  - Minimal by default: a restrained color palette, hardly any icons, and secondary actions hidden behind hover, the keyboard, or the command palette.

## 8. Customization & Accessibility

- Themes: dark and light, plus custom colors, custom icons per object/tag, and a custom CSS override.
- Background image: any user-provided image, with blur and dim controls.
- Keyboard: every action can be bound to a custom shortcut, plus an optional **vim mode** (navigation and editor).
- Toggleable modules (habits, goals, gamification, journal, ML, Telegram, ...). Disabled modules are hidden everywhere.
- Navigation and sidebar can be reordered, hidden, and given pinned views.
- Accessibility: WCAG AA contrast, full keyboard navigation, screen-reader labels, font-size scaling, reduced-motion option.

## 9. Security & Self-Hosting

- Single owner account: password (argon2) with optional TOTP 2FA. Clients and the CLI authenticate with session or API tokens.
- HTTPS through a reverse proxy (documented), or optional built-in TLS.
- No app-level encryption at rest. Users rely on their own disk encryption (documented).
- Automatic daily SQLite backups with rotation, plus one-command restore.
- Install options: Docker Compose, a single binary, or a systemd unit. The first run starts a setup wizard.
- Config via `config.toml` and env vars.

## 10. Non-Functional

- Server idle memory under 150 MB, excluding loaded ML models.
- Runs on a 2 vCPU / 2 GB VM.
- Schema migrations are versioned and run automatically on upgrade.
- Public API is documented (OpenAPI), so users can write their own scripts and integrations.
- Tests: unit tests (parser, recurrence, sync), API integration tests, and end-to-end UI tests for the core flows.
