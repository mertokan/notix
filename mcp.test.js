// node --test — mcp.js'i gercek bir istemci gibi stdio uzerinden surer.
const test = require('node:test')
const assert = require('node:assert')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { spawn } = require('node:child_process')
const readline = require('node:readline')

test('mcp: initialize → tools → add/read/complete/409/due', async (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'notix-mcp-'))
  const p = spawn(process.execPath, [path.join(__dirname, 'mcp.js')], {
    env: { ...process.env, NOTIX_DIR: dir, NOTIX_BACKUP_MS: '3600000' }, // yedek bu surede tetiklenmez
    stdio: ['pipe', 'pipe', 'inherit'],
  })
  t.after(() => p.kill())
  const bekleyen = new Map()
  readline.createInterface({ input: p.stdout }).on('line', (l) => {
    const m = JSON.parse(l)
    bekleyen.get(m.id)?.(m)
    bekleyen.delete(m.id)
  })
  let n = 0
  const rpc = (method, params) => new Promise((ok) => {
    const id = ++n
    bekleyen.set(id, ok)
    p.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n')
  })
  const call = async (name, args) => (await rpc('tools/call', { name, arguments: args })).result

  const init = await rpc('initialize', { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 't', version: '0' } })
  assert.equal(init.result.protocolVersion, '2025-06-18')
  assert.equal(init.result.serverInfo.name, 'notix')
  assert.ok(init.result.capabilities.tools)
  assert.equal((await rpc('initialize', { protocolVersion: '1999-01-01' })).result.protocolVersion, '2025-06-18')
  p.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n') // cevapsiz

  const names = (await rpc('tools/list')).result.tools.map((x) => x.name)
  for (const x of ['list_projects', 'read_project', 'add_task', 'complete_task', 'due_tasks', 'write_memory']) assert.ok(names.includes(x), x)
  assert.ok(!names.some((x) => /delete|remove/.test(x)))
  assert.equal((await rpc('nope/x')).error.code, -32601)

  assert.ok((await call('add_task', { slug: 'ev', text: 'yok' })).isError) // proje yok, create yok
  assert.ok(!(await call('add_task', { slug: 'ev', text: 'fatura öde @2000-01-01', create: true })).isError)

  const proj = JSON.parse((await call('read_project', { slug: 'ev' })).content[0].text)
  assert.match(proj.markdown, /- \[ \] fatura öde/)
  const [task] = proj.tasks
  assert.equal(task.text, 'fatura öde @2000-01-01')

  const due = JSON.parse((await call('due_tasks', {})).content[0].text)
  assert.deepEqual(due.map((x) => [x.slug, x.i]), [['ev', task.line]])

  assert.ok(!(await call('complete_task', { slug: 'ev', line: task.line, expect: task.text })).isError)
  assert.match(fs.readFileSync(path.join(dir, 'ev.md'), 'utf8'), /- \[x\] fatura öde/)

  const bad = await call('complete_task', { slug: 'ev', line: task.line, expect: 'eski metin' })
  assert.ok(bad.isError)
  assert.match(bad.content[0].text, /re-read/)

  assert.deepEqual(JSON.parse((await call('due_tasks', { date: '2099-01-01' })).content[0].text), []) // bitmis is gelmez

  assert.ok(!(await call('write_memory', { slug: 'ev', text: 'kira ayin 5i', expect: '' })).isError)
  assert.ok((await call('write_memory', { slug: 'ev', text: 'x', expect: '' })).isError) // arada degisti
  assert.equal(JSON.parse((await call('read_memory', { slug: 'ev' })).content[0].text).text, 'kira ayin 5i')
})
