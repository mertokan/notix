const http = require('node:http')
const fs = require('node:fs')
const path = require('node:path')
const crypto = require('node:crypto')
const store = require('./store')

const PORT = Number(process.env.NOTIX_PORT || 4321)
const PUBLIC = path.join(__dirname, 'public')
// Kurulu uygulamada __dirname salt okunur (app.asar); token/ayar oraya yazilamaz.
// electron.js paketliyken NOTIX_HOME=userData verir.
const HOME = process.env.NOTIX_HOME || __dirname
const VERSION = require('./package.json').version
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.webmanifest': 'application/manifest+json' }

// Sadece 127.0.0.1'e baglaniyoruz; token yine de duruyor cunku localhost dinleyen
// bir porta tarayicidaki herhangi bir sayfa da POST atabilir (DNS rebinding).
const TOKEN = process.env.NOTIX_TOKEN || readToken()
function readToken() {
  const f = path.join(HOME, '.notix-token')
  if (!fs.existsSync(f)) fs.writeFileSync(f, crypto.randomBytes(5).toString('hex'))
  return fs.readFileSync(f, 'utf8').trim()
}

// Kisayollar. Global olani (quickAdd) Electron kaydeder, gerisi tarayici tarafinda.
const CFG = path.join(HOME, '.notix-config.json')
const KEYS = { quickAdd: 'Control+Alt+N', find: 'Control+K', add: 'N', hideDone: 'H', help: '?' }
// Masaustu secenekleri: acilista baslat (Electron setLoginItemSettings), guncelleme
// denetimi, otomatik yedek (notes/ icinde git commit).
const OPTS = { openAtLogin: false, autoUpdate: true, backup: true }
const DEF = { ...KEYS, ...OPTS }
const config = () => { try { return { ...DEF, ...JSON.parse(fs.readFileSync(CFG, 'utf8')) } } catch { return { ...DEF } } }
function writeConfig(body) {
  const next = {} // sadece bildigimiz alanlar: kisayollar makul uzunlukta string, secenekler boolean
  for (const k of Object.keys(KEYS)) next[k] = typeof body[k] === 'string' && body[k].trim() && body[k].length <= 40 ? body[k].trim() : KEYS[k]
  for (const k of Object.keys(OPTS)) next[k] = typeof body[k] === 'boolean' ? body[k] : OPTS[k]
  fs.writeFileSync(CFG, JSON.stringify(next, null, 2))
  store.setBackup(next.backup)
  return next
}
store.setBackup(config().backup)
let configHook = null // electron.js global kisayoyu/acilista baslatmayi yeniden uygulasin diye
const onConfig = (fn) => (configHook = fn)

// Guncelleme denetimi: GitHub'in son surum API'si (hesap/sunucu gerekmez). Simdilik
// sadece "yeni surum var" der; indirme/kurma electron-updater ile sonra.
// ponytail: surum karsilastirmasi sayisal parcalarla, semver on-eki yok.
const UPDATE_URL = process.env.NOTIX_UPDATE_URL || 'https://api.github.com/repos/mertokan/notix/releases/latest'
const newer = (a, b) => { const A = a.split('.').map(Number), B = b.split('.').map(Number); for (let i = 0; i < 3; i++) if ((A[i] || 0) !== (B[i] || 0)) return (A[i] || 0) > (B[i] || 0); return false }
async function checkUpdate() {
  const r = await fetch(UPDATE_URL, { headers: { accept: 'application/vnd.github+json' }, signal: AbortSignal.timeout(5000) })
  if (!r.ok) throw fail(502, `guncelleme sunucusu ${r.status}`)
  const j = await r.json()
  const latest = String(j.tag_name || j.version || '').replace(/^v/, '')
  return { current: VERSION, latest, url: j.html_url || null, available: !!latest && newer(latest, VERSION) }
}

// Hizli ekleme kutusu proje listesini acinca pencere buyusun diye. Ayri bir IPC
// kurmak yerine zaten ayni process'te olan sunucu uzerinden.
let windowHook = null
const onWindow = (fn) => (windowHook = fn)

// Not klasorunu isletim sisteminde acmak. Sayfa file:// baglantisi acamiyor
// (Chromium http sayfasindan engelliyor), Electron shell.openPath yapiyor.
let openHook = null
const onOpen = (fn) => (openHook = fn)

const fail = (code, msg) => Object.assign(new Error(msg), { code })
// Yerel gun (toISOString UTC verir, gece yarisi civari yanlis gune dusebilir).
const yerelBugun = () => { const d = new Date(); return new Date(d - d.getTimezoneOffset() * 6e4).toISOString().slice(0, 10) }

function send(res, code, body, type = 'application/json') {
  res.writeHead(code, { 'content-type': type, 'cache-control': 'no-store' })
  res.end(type === 'application/json' ? JSON.stringify(body) : body)
}

async function readBody(req) {
  let s = ''
  for await (const c of req) { s += c; if (s.length > 8e6) throw fail(413, 'cok buyuk') } // ice aktarma paketi sigsin
  return s ? JSON.parse(s) : {}
}

async function api(req, res, url) {
  if ((req.headers['x-notix-token'] || url.searchParams.get('t')) !== TOKEN) return send(res, 401, { error: 'token yanlis' })
  const [, kind, slug, sub, idx] = url.pathname.split('/')
  const m = req.method
  const expect = url.searchParams.get('expect')

  if (kind === 'info') return send(res, 200, { dir: store.DIR, version: VERSION })
  if (kind === 'update') return send(res, 200, await checkUpdate())
  if (kind === 'search') return send(res, 200, store.search(url.searchParams.get('q') || ''))
  // Bugun = istemcinin yerel tarihi (d=YYYY-MM-DD); verilmezse sunucununki.
  if (kind === 'due') return send(res, 200, store.due(/^\d{4}-\d{2}-\d{2}$/.test(url.searchParams.get('d') || '') ? url.searchParams.get('d') : yerelBugun()))
  if (kind === 'export') return send(res, 200, { version: 1, files: store.exportAll() })
  if (kind === 'import') {
    if (m !== 'POST') return send(res, 405, { error: 'yontem desteklenmiyor' })
    const { files } = await readBody(req)
    if (!files || typeof files !== 'object') throw fail(400, 'files bekleniyor')
    const done = Object.entries(files).map(([s, text]) => store.importFile(s, text))
    return send(res, 200, { written: done.length, replaced: done.filter((d) => d.replaced).map((d) => d.slug) })
  }
  if (kind === 'media') {
    if (m !== 'POST') return send(res, 405, { error: 'yontem desteklenmiyor' })
    const b = await readBody(req)
    return send(res, 200, { path: store.saveMedia(b.name, b.data) })
  }
  if (kind === 'open') {
    if (m !== 'POST') return send(res, 405, { error: 'yontem desteklenmiyor' })
    return send(res, 200, { ok: !!openHook && openHook(store.DIR) })
  }
  if (kind === 'window') {
    if (m !== 'POST') return send(res, 405, { error: 'yontem desteklenmiyor' })
    const h = Number((await readBody(req)).height)
    return send(res, 200, { ok: !!windowHook && Number.isFinite(h) && windowHook(h) })
  }
  if (kind === 'config') {
    if (m === 'GET') return send(res, 200, { keys: config(), desktop: !!configHook })
    if (m === 'PUT') { const keys = writeConfig(await readBody(req)); return send(res, 200, { keys, bound: configHook ? configHook(keys) : null }) }
    return send(res, 405, { error: 'yontem desteklenmiyor' })
  }
  if (kind === 'trash') {
    const name = slug && decodeURIComponent(slug)
    if (!name && m === 'GET') return send(res, 200, store.trash())
    if (name && sub === 'restore' && m === 'POST') return send(res, 200, { slug: store.restore(name) })
    if (name && !sub && m === 'DELETE') { store.purge(name); return send(res, 200, { ok: true }) }
    return send(res, 405, { error: 'yontem desteklenmiyor' })
  }
  if (kind !== 'projects') return send(res, 404, { error: 'yok' })

  if (!slug) {
    if (m === 'GET') return send(res, 200, store.list())
    if (m === 'POST') return send(res, 200, { slug: store.create((await readBody(req)).title) })
  } else if (!sub) {
    if (m === 'GET') return send(res, 200, store.parse(slug))
    if (m === 'PATCH') return send(res, 200, { slug: store.rename(slug, (await readBody(req)).title) })
    if (m === 'DELETE') { store.remove(slug); return send(res, 200, { ok: true }) }
  } else if (sub === 'memory') {
    if (m === 'GET') return send(res, 200, store.memory(slug))
    if (m === 'PUT') { const b = await readBody(req); store.setMemory(slug, b.text, b.expect); return send(res, 200, store.memory(slug)) }
  } else if (sub === 'raw') {
    if (m === 'GET') return send(res, 200, { text: store.read(slug) })
    if (m === 'PUT') { const b = await readBody(req); store.write(slug, b.text, b.expect); return send(res, 200, store.parse(slug)) }
  } else if (sub === 'tasks') {
    if (m === 'POST') { const b = await readBody(req); store.addTask(slug, b.text, b.kind); return send(res, 200, store.parse(slug)) }
    // Ayni yol: `move` yukari/asagi, `to` baska projeye, gerisi satir duzenleme.
    if (m === 'PATCH') {
      const b = await readBody(req)
      if (b.move) store.moveTask(slug, Number(idx), b.move === 'up' ? 'up' : 'down', b.expect)
      else if (b.to) store.moveToProject(slug, Number(idx), b.to, b.expect)
      else store.updateTask(slug, Number(idx), b)
      return send(res, 200, store.parse(slug))
    }
    if (m === 'DELETE') { store.updateTask(slug, Number(idx), { expect, remove: true }); return send(res, 200, store.parse(slug)) }
  }
  send(res, 405, { error: 'yontem desteklenmiyor' })
}

function serveStatic(res, pathname) {
  const f = path.join(PUBLIC, path.normalize(pathname === '/' ? '/index.html' : pathname))
  if (!f.startsWith(PUBLIC)) return send(res, 403, 'no', 'text/plain')
  fs.readFile(f, (e, d) => (e ? send(res, 404, 'yok', 'text/plain') : send(res, 200, d, MIME[path.extname(f)] || 'application/octet-stream')))
}

// notes/media/ icindeki resimler. <img src> baslik gonderemedigi icin token ?t= ile.
function serveMedia(res, url) {
  if (url.searchParams.get('t') !== TOKEN) return send(res, 401, 'token yanlis', 'text/plain')
  try {
    const f = store.mediaFile(decodeURIComponent(url.pathname.slice(7)))
    fs.readFile(f.path, (e, d) => (e ? send(res, 404, 'yok', 'text/plain') : send(res, 200, d, f.type)))
  } catch (e) {
    send(res, e.code || 500, 'olmadi', 'text/plain')
  }
}

const handler = (req, res) => {
  const url = new URL(req.url, 'http://x')
  if (url.pathname.startsWith('/media/')) return serveMedia(res, url)
  if (!url.pathname.startsWith('/api/')) return serveStatic(res, url.pathname)
  api(req, res, new URL(req.url.replace('/api', ''), 'http://x')).catch((e) =>
    send(res, e.code >= 400 && e.code < 600 ? e.code : 500, { error: e.message }))
}

function start() {
  return http.createServer(handler).listen(PORT, '127.0.0.1', () =>
    console.log(`notix: http://127.0.0.1:${PORT}/?t=${TOKEN}`))
}

module.exports = { start, PORT, TOKEN, config, onConfig, onWindow, onOpen, newer }
if (require.main === module) start()
