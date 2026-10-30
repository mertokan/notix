# Notix — instructions for AI agents

Notix's data source is **plain markdown files, not an API**: `notes/<project-slug>.md`
(in the installed app, the notes folder shown at the bottom left of the app). Read
and edit the files directly. The app picks up changes instantly.

No file access? Use the MCP server (`mcp.js`; Settings → AI agents → Copy MCP config)
or the HTTP API below.

## Format

```markdown
# Project name

Free text lines show up as notes. **Bold**, `code`, [links](https://…)
and ![images](media/file.png) render in the app too.

- [ ] open task
- [x] done task
  - [ ] indented sub-task (indentation is kept)
- [ ] task with a date @2026-09-15

## Memory

The project's memory. Shown in its own panel at the top of the project in the app.
```

Rules:
- `- [ ]` = open, `- [x]` = done. Do not use any other marker.
- Dates go inside the line as `@YYYY-MM-DD`. Open tasks dated today or earlier are
  collected in the app's "Today" list. Do not use any other date format.
- **Edit line by line.** Leave lines you don't recognize (headings, blank lines,
  free text) exactly as they are.
- When you finish a task, don't delete the line. Change `[ ]` to `[x]` so the history stays.
- New task = a new `- [ ]` line at the end of the file. If the file ends with a
  `## Memory` (or `## Bellek`) section, add the task **above** it.
- New project = `notes/<slug>.md`, first line `# Project name`. Slug: lowercase
  letters, digits and `-`.
- Don't delete files. The app moves deletions into `notes/.trash/`; do the same.

## Project memory

The section under a `## Memory` heading (`## Bellek` in Turkish files; both work)
is the project's memory: decisions, context, links, open questions. It sits at the
**end** of the file (the section runs to the next heading of the same or higher
level, or to the end of the file).

When working on a project, read this first. Write lasting knowledge here (why
something was done a certain way, which approach was tried and dropped, where
things live), not into the task list. Don't delete existing lines; add or update.

## Typical flow

1. Read the files under `notes/`, take context from the memory section, and look
   at the open (`- [ ]`) tasks.
2. Do the work.
3. Mark the line `- [x]`; add an explanation line below it if useful. If you
   learned something lasting, write it to the memory section.

## HTTP API (for agents without file access)

When the server is running (`npm run server`, only on `127.0.0.1:4321`), every
request needs the header `x-notix-token: <contents of the .notix-token file>`.

| | |
|---|---|
| `GET /api/projects` | project list + open task counts |
| `GET /api/projects/:slug` | content line by line (`i` = line number) |
| `POST /api/projects` | `{title}` |
| `PATCH /api/projects/:slug` | `{title}` — renames the heading line and the file |
| `POST /api/projects/:slug/tasks` | `{text, kind?}` — `kind`: `task` (default), `sub` (indented), `note` (free text) |
| `PATCH /api/projects/:slug/tasks/:i` | `{expect, done?, text?}` — `{expect, move:"up"\|"down"}` reorders, `{expect, to:"target-slug"}` moves to another project |
| `GET /api/search?q=` | search lines in all projects (at least 2 characters) |
| `GET /api/due?d=YYYY-MM-DD` | open tasks due up to that day (all projects) |
| `DELETE /api/projects/:slug/tasks/:i?expect=…` | |
| `GET /api/export` | `{version, files: {slug: content}}` |
| `POST /api/import` | `{files: {slug: content}}` — an existing file is copied to `.trash` first |
| `GET/PUT /api/config` | settings |
| `GET /api/projects/:slug/memory` | the memory section (`{text, from, to}`) |
| `PUT /api/projects/:slug/memory` | `{text, expect, heading?}` — write the section (created if missing with `heading` `Memory`/`Bellek`; empty text removes it) |
| `GET /api/projects/:slug/raw` | the whole file (`{text}`) |
| `PUT /api/projects/:slug/raw` | `{text, expect}` — write the whole file; `expect` is the content you read, 409 if it differs |
| `POST /api/media` | `{name, data}` (base64 image) → `{path: "media/…"}` |
| `GET /api/trash` | files in the trash |
| `POST /api/trash/:file/restore` | restore (on a name clash it becomes `slug-2`, never overwrites) |
| `DELETE /api/trash/:file` | permanent delete — the only real delete in the app, cannot be undone |

`expect` = the current text of the line you want to change. If it doesn't match you
get `409`: the file changed in the meantime, read it again.
