# Notix

**English** · [Türkçe](README.tr.md)

A to-do list your AI agents can actually use.

Notix is a local desktop app for project notes and tasks. There is no database and
no account: every project is a plain markdown file (`- [ ]` / `- [x]`) in a folder you
choose. That means Claude Code, Cursor, Codex or any agent with file access can read
your list and tick items off. You watch it happen live in the app. Agents without
file access connect through the built-in **MCP server**.

<!-- TODO: 10s demo GIF — agent ticks a task, the app lights up the line -->

## Why

- **Plain markdown.** Your notes stay readable in any editor, diffable, and yours.
- **Agent-native.** Agents follow [AGENTS.md](AGENTS.md), and a `## Memory` (or `## Bellek`) section
  in each project keeps decisions and context between sessions.
- **Live.** Changes made from outside show up instantly, and the changed line is
  highlighted.
- **Safe for concurrent writes.** Every edit carries the line it expects to change.
  If an agent changed it in the meantime, nothing gets overwritten.
- **Nothing is lost.** Deleted projects go to `.trash/`, and the notes folder is
  its own git repo with automatic commits.
- **Quick capture.** `Ctrl+Alt+N` from anywhere drops a line into Inbox.
- **Due dates.** Write `@2026-09-15` in a line. Due and overdue tasks gather under
  **Today** and trigger a desktop notification.
- **Yours to customize.** 9 languages, 6 themes (dark, light, system, midnight,
  paper, forest), 8 accent colors, and rebindable shortcuts.

## Install

Download the installer from [Releases](https://github.com/mertokan/notix/releases)
(Windows `.exe`, macOS `.dmg`, Linux `.AppImage`).

From source:

```bash
npm install
npm start            # desktop app
npm run server       # server only, use it in a browser (127.0.0.1:4321)
npm test
npm run dist         # installer for the current platform → dist/
```

## Connect an AI agent

**Agents with file access** (Claude Code, Cursor, Codex…): point them at the notes
folder and [AGENTS.md](AGENTS.md). Nothing else is needed.

**MCP** (Claude Desktop, Cursor, any MCP client): open **⚙ Settings → AI agents → Copy
MCP config** and paste it into your client's `mcpServers`. From source:

```json
{
  "mcpServers": {
    "notix": {
      "command": "node",
      "args": ["/path/to/notix/mcp.js"],
      "env": { "NOTIX_DIR": "/path/to/notes" }
    }
  }
}
```

Claude Code: `claude mcp add notix -e NOTIX_DIR=/path/to/notes -- node /path/to/notix/mcp.js`

Tools: `list_projects`, `read_project`, `add_task`, `complete_task`, `reopen_task`,
`update_task`, `due_tasks`, `search`, `read_memory`, `write_memory`, `create_project`.
There is no delete tool on purpose.

## File format

```markdown
# Project name

Free text is shown as a note.

- [ ] open task @2026-09-15
- [x] done task
  - [ ] nested sub-task

## Memory

Project memory: decisions, context, links.
```

## Environment

| | |
|---|---|
| `NOTIX_DIR` | notes folder (default `./notes`; installed app: `Documents/Notix`, or the folder chosen in Settings) |
| `NOTIX_HOME` | where the token and settings live |
| `NOTIX_PORT` | default `4321` |
| `NOTIX_BACKUP_MS` | delay before the automatic git commit, default `20000` |

The server listens only on `127.0.0.1`, and every API call needs a per-install token.

## License

<!-- TODO: choose a license (MIT?) before the first public release -->
