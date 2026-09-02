// Kaynak = notes/<slug>.md. Satir-tabanli: tanimadigimiz satirlara dokunmayiz,
// boylece AI'in/kullanicinin elle yazdigi her sey aynen korunur.
const fs = require('node:fs')
const path = require('node:path')

const DIR = process.env.NOTIX_DIR || path.join(__dirname, 'notes')
const TRASH = path.join(DIR, '.trash')
const TASK = /^(\s*)- \[( |x|X)\] ?(.*)$/

const bad = (code, msg) => Object.assign(new Error(msg), { code })
// Dosya adlari ASCII kalsin ama "Ev İşleri" → ev-isleri olsun, ev-i-leri degil.
const TR = { ç: 'c', Ç: 'c', ğ: 'g', Ğ: 'g', ı: 'i', I: 'i', İ: 'i', ö: 'o', Ö: 'o', ş: 's', Ş: 's', ü: 'u', Ü: 'u' }
const slugify = (s) =>
  String(s).replace(/[çÇğĞıIİöÖşŞüÜ]/g, (c) => TR[c])
    .toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 60)

function file(slug) {
  if (!/^[a-z0-9][a-z0-9-]*$/.test(slug)) throw bad(400, 'gecersiz proje adi')
  return path.join(DIR, slug + '.md')
}

const readLines = (slug) => fs.readFileSync(file(slug), 'utf8').split(/\r?\n/)
const writeLines = (slug, lines) => fs.writeFileSync(file(slug), lines.join('\n'))

function parse(slug) {
  const items = readLines(slug).map((raw, i) => {
    const m = TASK.exec(raw)
    if (m) return { i, kind: 'task', done: m[2] !== ' ', text: m[3] }
    const h = /^(#{1,6}) (.*)$/.exec(raw)
    if (h) return { i, kind: 'heading', level: h[1].length, text: h[2] }
    return { i, kind: 'text', text: raw }
  })
  return { slug, title: items.find((x) => x.kind === 'heading')?.text || slug, items }
}

function list() {
  fs.mkdirSync(DIR, { recursive: true })
  return fs
    .readdirSync(DIR)
    .filter((f) => f.endsWith('.md'))
    .map((f) => {
      const p = parse(f.slice(0, -3))
      const t = p.items.filter((x) => x.kind === 'task')
      return { slug: p.slug, title: p.title, open: t.filter((x) => !x.done).length, done: t.length - t.filter((x) => !x.done).length }
    })
    .sort((a, b) => b.open - a.open || a.title.localeCompare(b.title))
}

// --- cop kutusu: silinen projeler ve ice aktarmada uzerine yazilanlar burada ---
const trashPath = (name) => {
  if (!/^[a-z0-9][a-z0-9-]*\.md$/.test(name)) throw bad(400, 'gecersiz dosya')
  return path.join(TRASH, name)
}

function trash() {
  if (!fs.existsSync(TRASH)) return []
  return fs.readdirSync(TRASH).filter((f) => f.endsWith('.md')).map((f) => {
    const raw = fs.readFileSync(path.join(TRASH, f), 'utf8')
    const tasks = raw.split(/\r?\n/).map((l) => TASK.exec(l)).filter(Boolean)
    return {
      file: f,
      title: /^#{1,6} (.*)$/m.exec(raw)?.[1] || f,
      at: fs.statSync(path.join(TRASH, f)).mtimeMs,
      open: tasks.filter((m) => m[2] === ' ').length,
      done: tasks.filter((m) => m[2] !== ' ').length,
    }
  }).sort((a, b) => b.at - a.at)
}

// Ayni adda proje varsa uzerine yazmaz, bir sonraki bos ada geri alir.
function restore(name) {
  const from = trashPath(name)
  const base = name.replace(/-\d+\.md$/, '').replace(/\.md$/, '')
  if (!base) throw bad(400, 'gecersiz dosya')
  let slug = base
  for (let n = 2; fs.existsSync(file(slug)); n++) slug = `${base}-${n}`
  fs.mkdirSync(DIR, { recursive: true })
  fs.renameSync(from, file(slug))
  return slug
}

const purge = (name) => fs.unlinkSync(trashPath(name)) // uygulamadaki tek gercek silme

function create(title) {
  const slug = slugify(title)
  if (!slug) throw bad(400, 'baslik bos')
  fs.mkdirSync(DIR, { recursive: true })
  if (!fs.existsSync(file(slug))) fs.writeFileSync(file(slug), `# ${String(title).trim()}\n\n`)
  return slug
}

function remove(slug) {
  fs.mkdirSync(TRASH, { recursive: true })
  fs.renameSync(file(slug), path.join(TRASH, `${slug}-${Date.now()}.md`)) // silme yok, tasima
}

// kind: 'task' (varsayilan) | 'sub' (girintili alt is) | 'note' (serbest metin)
function addTask(slug, text, kind) {
  text = String(text).replace(/[\r\n]+/g, ' ').trim()
  if (!text) throw bad(400, 'bos gorev')
  const lines = readLines(slug)
  const satir = kind === 'note' ? text : `${kind === 'sub' ? '  ' : ''}- [ ] ${text}`
  const bellek = memRange(lines)
  if (bellek) { // bellek en altta duruyor, yeni satir onun ustune
    let i = bellek.bas
    while (i > 0 && !lines[i - 1].trim()) i--
    lines.splice(i, 0, satir)
  } else {
    while (lines.length && !lines[lines.length - 1].trim()) lines.pop()
    lines.push(satir, '')
  }
  writeLines(slug, lines)
}

// Basligi ve dosya adini degistirir; geri kalan satirlara dokunmaz.
function rename(slug, title) {
  const t = String(title).replace(/[\r\n]+/g, ' ').trim()
  const next = slugify(t)
  if (!next) throw bad(400, 'baslik bos')
  if (next !== slug && fs.existsSync(file(next))) throw bad(409, 'bu isimde proje var')
  const lines = readLines(slug)
  const i = lines.findIndex((l) => /^#{1,6} /.test(l))
  if (i < 0) lines.unshift(`# ${t}`)
  else lines[i] = lines[i].replace(/^(#{1,6}) .*/, (_, h) => `${h} ${t}`)
  writeLines(slug, lines)
  if (next !== slug) fs.renameSync(file(slug), file(next))
  return next
}

function search(q) {
  q = String(q).trim().toLowerCase()
  if (q.length < 2) return []
  return list().flatMap(({ slug, title }) =>
    parse(slug).items
      .filter((x) => x.kind !== 'heading' && x.text.trim() && x.text.toLowerCase().includes(q))
      .map((x) => ({ slug, project: title, i: x.i, kind: x.kind, done: x.done, text: x.text }))
  ).slice(0, 200)
}

const exportAll = () => {
  fs.mkdirSync(DIR, { recursive: true })
  return Object.fromEntries(fs.readdirSync(DIR).filter((f) => f.endsWith('.md'))
    .map((f) => [f.slice(0, -3), fs.readFileSync(path.join(DIR, f), 'utf8')]))
}

// Ustune yazmadan once eskisini .trash'e kopyalar — ice aktarma veri kaybettirmesin.
function importFile(slug, text) {
  const s = slugify(slug)
  if (!s) throw bad(400, 'gecersiz dosya adi')
  if (typeof text !== 'string') throw bad(400, 'icerik metin olmali')
  fs.mkdirSync(DIR, { recursive: true })
  const f = file(s)
  const replaced = fs.existsSync(f)
  if (replaced) {
    fs.mkdirSync(TRASH, { recursive: true })
    fs.copyFileSync(f, path.join(TRASH, `${s}-${Date.now()}.md`))
  }
  fs.writeFileSync(f, text.replace(/\r\n/g, '\n'))
  return { slug: s, replaced }
}

// --- proje bellegi: "## Bellek" basligi altindaki satirlar. Ayri dosya degil,
// cunku ajanlar zaten proje dosyasini okuyor. Bolum, ayni/ust seviye baska bir
// baslik baslayinca biter. ---
const MEM = /^(#{2,6}) *(bellek|memory) *$/i

function memRange(lines) {
  const bas = lines.findIndex((l) => MEM.test(l))
  if (bas < 0) return null
  const seviye = MEM.exec(lines[bas])[1].length
  let son = bas + 1
  for (; son < lines.length; son++) {
    const h = /^(#{1,6}) /.exec(lines[son])
    if (h && h[1].length <= seviye) break
  }
  return { bas, son }
}

function memory(slug) {
  const lines = readLines(slug)
  const r = memRange(lines)
  if (!r) return { text: '', from: -1, to: -1 }
  return { text: lines.slice(r.bas + 1, r.son).join('\n').replace(/^\n+|\n+$/g, ''), from: r.bas, to: r.son }
}

// expect = istemcinin gordugu bellek metni; arada AI yazdiysa 409.
function setMemory(slug, text, expect) {
  if (typeof text !== 'string') throw bad(400, 'icerik metin olmali')
  const simdi = memory(slug)
  if (expect != null && simdi.text !== expect) throw bad(409, 'bellek arada degismis')
  const lines = readLines(slug)
  const govde = text.replace(/\r\n/g, '\n').replace(/^\n+|\n+$/g, '')

  if (simdi.from >= 0) {
    // bos birakildiysa bolumu tamamen kaldir
    lines.splice(simdi.from, simdi.to - simdi.from, ...(govde ? ['## Bellek', '', ...govde.split('\n'), ''] : []))
  } else if (govde) {
    // Bolum dosyanin sonuna: "sonraki basliga kadar" kurali yuzunden ortada
    // olsa altindaki her seyi yutardi. Yeni isler de bunun ustune girer.
    while (lines.length && !lines[lines.length - 1].trim()) lines.pop()
    lines.push('', '## Bellek', '', ...govde.split('\n'), '')
  }
  writeLines(slug, lines)
}

// --- belge modu: dosyanin tamami. Not/bellek tutmak icin (cok satirli metin,
// baslik, resim) — satir satir eklenemeyen her sey buradan yazilir. ---
const read = (slug) => fs.readFileSync(file(slug), 'utf8')

// expect = kullanicinin duzenlemeye basladigi andaki icerik. Arada AI/editor
// yazdiysa uymaz ve 409 doner; yazdiklarini ezmeyiz.
function write(slug, text, expect) {
  if (typeof text !== 'string') throw bad(400, 'icerik metin olmali')
  if (expect != null && read(slug) !== expect) throw bad(409, 'dosya arada degismis')
  fs.writeFileSync(file(slug), text.replace(/\r\n/g, '\n'))
}

// --- resimler: notes/media/ ---
const MEDIA = path.join(DIR, 'media')
const IMG = { png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp', svg: 'image/svg+xml' }

function saveMedia(name, base64) {
  const ext = String(name).split('.').pop().toLowerCase()
  if (!IMG[ext]) throw bad(400, 'sadece resim dosyasi')
  const buf = Buffer.from(String(base64), 'base64')
  if (!buf.length) throw bad(400, 'bos dosya')
  if (buf.length > 5e6) throw bad(413, 'resim 5MB dan buyuk')
  fs.mkdirSync(MEDIA, { recursive: true })
  const f = `${slugify(String(name).replace(/\.[^.]+$/, '')) || 'resim'}-${Date.now()}.${ext}`
  fs.writeFileSync(path.join(MEDIA, f), buf)
  return `media/${f}`
}

// Disaridan gelen ad: sadece bildigimiz uzantilar, klasor disina cikamaz.
function mediaFile(name) {
  const m = /^([a-z0-9][a-z0-9-]*)\.([a-z]+)$/i.exec(name)
  if (!m || !IMG[m[2].toLowerCase()]) throw bad(400, 'gecersiz dosya')
  return { path: path.join(MEDIA, name), type: IMG[m[2].toLowerCase()] }
}

// expect: istemcinin gordugu metin. Uymuyorsa dosya arada degismis (AI yazmis) -> 409.
function updateTask(slug, i, { expect, done, text, remove: rm }) {
  const lines = readLines(slug)
  const m = TASK.exec(lines[i] ?? '')
  if (!m) throw bad(409, 'satir degismis')
  if (expect != null && m[3] !== expect) throw bad(409, 'satir degismis')
  if (rm) lines.splice(i, 1)
  else lines[i] = `${m[1]}- [${(done ?? m[2] !== ' ') ? 'x' : ' '}] ${text != null ? String(text).replace(/[\r\n]+/g, ' ').trim() : m[3]}`
  writeLines(slug, lines)
}

module.exports = { DIR, list, parse, read, write, memory, setMemory, create, remove, rename, addTask, updateTask, search, exportAll, importFile, trash, restore, purge, saveMedia, mediaFile, slugify }
