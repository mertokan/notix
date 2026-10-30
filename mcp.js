// Notix MCP sunucusu: stdio, satir ayrimli JSON-RPC 2.0, paket yok.
// store.js'i dogrudan kullanir — uygulama kapaliyken de calisir; aciksa ekran
// dosya izlemesiyle tazelenir. stdout SADECE protokol icin; log stderr'e.
const store = require('./store')
const { version } = require('./package.json')

const PROTOCOLS = ['2024-11-05', '2025-03-26', '2025-06-18', '2025-11-25']
const DATE = /^\d{4}-\d{2}-\d{2}$/
const today = () => { const d = new Date(); return new Date(d - d.getTimezoneOffset() * 6e4).toISOString().slice(0, 10) }

const str = (description) => ({ type: 'string', description })
const slug = str('Project slug (file name without .md), e.g. "home-chores". Get it from list_projects.')
const line = { type: 'integer', minimum: 0, description: 'Line index of the task, as shown by read_project.' }
const expect = str('Current text of the task (after "- [ ] "), exactly as read_project showed it. If the file changed since, the call fails: re-read.')
const tool = (name, description, properties, required = []) =>
  ({ name, description, inputSchema: { type: 'object', properties, required, additionalProperties: false } })

const TOOLS = [
  tool('list_projects', 'List all projects with open and done task counts.', {}),
  tool('read_project', 'Read a project: raw markdown file plus its tasks with line indexes (needed for complete/reopen/update). Read the "## Bellek" (memory) section first for context.', { slug }, ['slug']),
  tool('add_task', 'Append a line to a project (inserted above the memory section). A due date goes inline as "@YYYY-MM-DD".', {
    slug,
    text: str('Task text. Multi-line text is kept as-is; the first line gets the chosen kind.'),
    kind: { type: 'string', enum: ['task', 'sub', 'note'], description: 'task (default), sub = indented subtask, note = plain text line.' },
    create: { type: 'boolean', description: 'Create the project if it does not exist.' },
  }, ['slug', 'text']),
  tool('complete_task', 'Mark a task done ([x]). Never delete finished tasks.', { slug, line, expect }, ['slug', 'line', 'expect']),
  tool('reopen_task', 'Mark a done task open again ([ ]).', { slug, line, expect }, ['slug', 'line', 'expect']),
  tool('update_task', 'Replace the text of a task, keeping its done state and indentation.', { slug, line, expect, text: str('New task text (single line).') }, ['slug', 'line', 'expect', 'text']),
  tool('due_tasks', 'Open tasks across all projects whose @date is on or before the given day.', { date: str('YYYY-MM-DD; defaults to local today.') }),
  tool('search', 'Search lines (tasks and notes) across all projects, case-insensitive, min 2 characters.', { q: str('Search text.') }, ['q']),
  tool('read_memory', 'Read the project memory ("## Bellek" section): decisions, context, open questions.', { slug }, ['slug']),
  tool('write_memory', 'Replace the project memory section. Keep existing lines; add or update. Empty text removes the section.', {
    slug, text: str('Full new memory text (markdown, without the heading).'), expect: str('Memory text exactly as read_memory returned it ("" if none).'),
  }, ['slug', 'text', 'expect']),
  tool('create_project', 'Create a new project (no-op if it exists). Returns its slug.', { title: str('Project title, e.g. "Home Chores".') }, ['title']),
]

function need(a, k, type = 'string') {
  if (typeof a[k] !== type || (type === 'number' && !Number.isInteger(a[k]))) throw new Error(`"${k}" must be ${type === 'number' ? 'an integer' : 'a string'}`)
  return a[k]
}

const setDone = (done) => (a) => { store.updateTask(need(a, 'slug'), need(a, 'line', 'number'), { expect: need(a, 'expect'), done }); return 'ok' }

const CALLS = {
  list_projects: () => store.list(),
  read_project: (a) => {
    const p = store.parse(need(a, 'slug'))
    const tasks = p.items.filter((x) => x.kind === 'task').map(({ i, done, indent, text }) => ({ line: i, done, indent, text }))
    return { slug: p.slug, title: p.title, markdown: store.read(p.slug), tasks }
  },
  add_task: (a) => {
    let s = need(a, 'slug')
    if (a.create) s = store.create(s)
    store.addTask(s, need(a, 'text'), a.kind)
    return { slug: s, ok: true }
  },
  complete_task: setDone(true),
  reopen_task: setDone(false),
  update_task: (a) => { store.updateTask(need(a, 'slug'), need(a, 'line', 'number'), { expect: need(a, 'expect'), text: need(a, 'text') }); return 'ok' },
  due_tasks: (a) => {
    const d = a.date ?? today()
    if (!DATE.test(d)) throw new Error('"date" must be YYYY-MM-DD')
    return store.due(d)
  },
  search: (a) => store.search(need(a, 'q')),
  read_memory: (a) => ({ text: store.memory(need(a, 'slug')).text }),
  write_memory: (a) => { store.setMemory(need(a, 'slug'), need(a, 'text'), need(a, 'expect')); return 'ok' },
  create_project: (a) => ({ slug: store.create(need(a, 'title')) }),
}

function callTool({ name, arguments: args } = {}) {
  const f = CALLS[name]
  if (!f) return { content: [{ type: 'text', text: `unknown tool: ${name}` }], isError: true }
  try {
    const r = f(args || {})
    return { content: [{ type: 'text', text: typeof r === 'string' ? r : JSON.stringify(r, null, 2) }] }
  } catch (e) {
    const text = e.code === 409 ? 'file changed, re-read the project and retry'
      : e.code === 'ENOENT' ? 'project not found (use list_projects, or add_task with create:true)'
        : e.message
    return { content: [{ type: 'text', text }], isError: true }
  }
}

function handle(msg) {
  const { id, method, params } = msg
  switch (method) {
    case 'initialize': return {
      protocolVersion: PROTOCOLS.includes(params?.protocolVersion) ? params.protocolVersion : '2025-06-18',
      capabilities: { tools: {} },
      serverInfo: { name: 'notix', version },
    }
    case 'ping': return {}
    case 'tools/list': return { tools: TOOLS }
    case 'tools/call': return callTool(params)
  }
  if (id !== undefined) throw Object.assign(new Error(`method not found: ${method}`), { rpc: -32601 })
}

const send = (m) => process.stdout.write(JSON.stringify({ jsonrpc: '2.0', ...m }) + '\n')

require('node:readline').createInterface({ input: process.stdin }).on('line', (raw) => {
  if (!raw.trim()) return
  let msg
  try { msg = JSON.parse(raw) } catch { return send({ id: null, error: { code: -32700, message: 'parse error' } }) }
  if (!msg || typeof msg !== 'object' || Array.isArray(msg)) return send({ id: null, error: { code: -32600, message: 'invalid request' } })
  if (typeof msg.method !== 'string') return // istemcinin cevaplari; biz istek gondermiyoruz
  try {
    const result = handle(msg)
    if (msg.id !== undefined) send({ id: msg.id, result })
  } catch (e) {
    if (!e.rpc) console.error(e)
    if (msg.id !== undefined) send({ id: msg.id, error: { code: e.rpc || -32603, message: e.message } })
  }
})
