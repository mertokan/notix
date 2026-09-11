const q = new URLSearchParams(location.search)
if (q.get('t')) { localStorage.notixToken = q.get('t'); history.replaceState(null, '', location.pathname + location.hash) }
MD.setToken(localStorage.notixToken || '')

const api = async (p, o = {}) => {
  const r = await fetch('/api' + p, { ...o, headers: { 'content-type': 'application/json', 'x-notix-token': localStorage.notixToken || '' } })
  const b = await r.json().catch(() => ({}))
  if (!r.ok) throw Object.assign(new Error(b.error || r.status), { status: r.status })
  return b
}

function el(tag, props, ...kids) {
  const n = Object.assign(document.createElement(tag), props)
  for (const k of kids.flat()) if (k != null) n.append(k)
  return n
}
const $ = (id) => document.getElementById(id)

// Yazma kutulari textarea: uzun satir tek satira sikisip okunmaz olmasin, saran
// metin icinde istedigin yeri secip kopyalayabil. Enter yine kaydeder/gonderir,
// Shift+Enter alt satir acar (cok satirli yapistirma da boyle korunur).
const buyut = (ta) => { ta.style.height = '0'; ta.style.height = ta.scrollHeight + 'px' }
function kutu(props, enter) {
  const ta = el('textarea', { rows: 1, spellcheck: false, ...props })
  ta.addEventListener('input', () => buyut(ta))
  ta.addEventListener('keydown', (e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); enter(ta) } })
  return ta
}

let cur = null
let hi = null // aramadan gelince vurgulanacak satir
let toastT
function say(msg) {
  const t = $('toast')
  t.textContent = msg
  t.classList.add('on')
  clearTimeout(toastT)
  toastT = setTimeout(() => t.classList.remove('on'), 3200)
}
const oops = (e) => { say(e.status === 409 ? 'Dosya arada değişmişti, tazelendi.' : 'Olmadı: ' + e.message); open_(cur) }

// Yerel gun: sunucunun degil kullanicinin takvimi esas (toISOString UTC verir).
const bugun = () => { const d = new Date(); return new Date(d - d.getTimezoneOffset() * 6e4).toISOString().slice(0, 10) }

async function loadProjects() {
  const [list, bin, due] = await Promise.all([api('/projects'), api('/trash').catch(() => []), api('/due?d=' + bugun()).catch(() => [])])
  $('today').querySelector('.count').textContent = due.length ? String(due.length) : ''
  $('today').classList.toggle('clear', !due.length)
  $('projects').replaceChildren(...list.map((p) =>
    el('a', { className: 'proj' + (p.slug === cur ? ' on' : '') + (p.open ? '' : ' clear'), href: '#' + p.slug },
      el('span', { className: 'name', textContent: p.title }),
      el('span', { className: 'count', textContent: p.open ? String(p.open) : '' }))))
  $('bin').querySelector('.count').textContent = bin.length ? String(bin.length) : ''
  $('bin').classList.toggle('clear', !bin.length)
}

// Satirin kimligi metnidir (store'un `expect`'i de oyle). Poll ile gelen bir
// degisiklik = dosyayi baskasi yazmis; o satir amber yanar.
let snap = new Map()

async function open_(slug, poll) {
  if (!slug || finding() || editing) return
  if (slug !== cur) { hi = null; cur = slug } // vurgu sadece gelinen projede kalir
  const [p, mem] = await Promise.all([
    api('/projects/' + slug),
    api(`/projects/${slug}/memory`).catch(() => ({ text: '', from: -1, to: -1 })),
  ])
  const tasks = p.items.filter((x) => x.kind === 'task')
  const hot = new Set(poll ? tasks.filter((x) => snap.get(x.text) !== x.done).map((x) => x.text) : [])
  snap = new Map(tasks.map((x) => [x.text, x.done]))
  const open = tasks.filter((x) => !x.done).length
  const title = el('h2', { textContent: p.title, title: 'Adını değiştirmek için tıkla', onclick: () => renameProject(p, title) })

  $('main').replaceChildren(
    el('header', {},
      el('span', { className: 'hash', ariaHidden: 'true', textContent: '#' }),
      title,
      el('span', { className: 'meta', textContent: `${open} açık · ${tasks.length - open} bitti` }),
      el('button', { className: 'link', textContent: 'Belge', title: 'Markdown olarak düzenle (not, resim, başlık)', onclick: openDoc }),
      el('button', { className: 'link', textContent: hidden() ? 'Bitmişleri göster' : 'Bitmişleri gizle', onclick: toggleDone }),
      el('button', { className: 'link warn', textContent: 'Çöpe taşı', title: 'notes/.trash içine taşınır', onclick: () => trashProject(slug) })),
    bellekPaneli(mem),
    // bellek bolumunun satirlari listede tekrar gorunmesin
    el('ul', { className: 'tasks' }, p.items
      .filter((it) => !(mem.from >= 0 && it.i >= mem.from && it.i < mem.to))
      .map((it) => row(it, hot.has(it.text), it.i === p.items.find((x) => x.kind === 'heading')?.i))),
    el('form', { className: 'add', onsubmit: addTask },
      el('span', {}),
      el('select', { name: 'kind', className: 'box', title: 'Satır türü' },
        el('option', { value: 'task', textContent: '[ ]' }),
        el('option', { value: 'sub', textContent: '↳' }),
        el('option', { value: 'note', textContent: '¶' })),
      ekleKutusu(),
      el('button', { textContent: 'Ekle ⏎' })))

  if (!poll) $('main').querySelector('.hit')?.scrollIntoView({ block: 'center' })
  if (hot.size) say(`Dosya değişti · ${hot.size} satır`)
  loadProjects()
}

/* --- proje bellegi: dosyadaki "## Bellek" bolumu, ekranin en ustunde --- */
function bellekSatirlari(text) {
  return text.split('\n').map((satir) => {
    const h = /^(#{1,6}) (.*)$/.exec(satir)
    if (h) return el('p', { className: 'mh' }, MD.render(h[2]))
    if (!satir.trim()) return null
    const t = /^(\s*)- \[( |x|X)\] ?(.*)$/.exec(satir)
    if (t) return el('p', { className: 'mt' + (t[2] === ' ' ? '' : ' bitti') },
      el('span', { className: 'box', ariaHidden: 'true', textContent: t[2] === ' ' ? '[ ] ' : '[x] ' }), MD.render(t[3]))
    return el('p', {}, MD.render(satir.replace(/^\s*- /, '• ')))
  }).filter(Boolean)
}

function bellekPaneli(mem) {
  const govde = mem.text
    ? el('div', { className: 'mem-body' }, bellekSatirlari(mem.text))
    : el('p', { className: 'mem-bos', textContent: 'Boş. Projeye dair hatırlanması gerekenleri buraya yaz — kararlar, bağlam, bağlantılar. AI ajanları da bu bölümü okuyup yazabiliyor.' })

  return el('section', { className: 'mem' + (mem.text ? '' : ' bos') },
    el('div', { className: 'mem-bar' },
      el('span', { className: 'etiket', textContent: 'Bellek' }),
      el('span', { className: 'yol', textContent: mem.from >= 0 ? `## Bellek · satır ${mem.from + 1}` : 'dosyada henüz yok' }),
      el('button', { className: 'link', textContent: mem.text ? 'Düzenle' : 'Ekle', onclick: (e) => bellekDuzenle(e.target.closest('.mem'), mem) })),
    govde)
}

function bellekDuzenle(panel, mem) {
  const ta = el('textarea', { className: 'mem-edit', value: mem.text, spellcheck: false, placeholder: 'Karar, bağlam, hatırlanacak not…' })
  MD.bind(ta, post, say)
  ta.onkeydown = (e) => { if (e.key === 'Escape') open_(cur) }
  panel.replaceChildren(
    el('div', { className: 'mem-bar' },
      el('span', { className: 'etiket', textContent: 'Bellek' }),
      el('span', { className: 'yol', textContent: 'markdown · Ctrl+B, Ctrl+V ile resim' }),
      el('button', { className: 'link', textContent: 'Vazgeç', onclick: () => open_(cur) }),
      el('button', { className: 'link ok', textContent: 'Kaydet', onclick: () => bellekKaydet(ta, mem) })),
    ta)
  ta.focus()
}

async function bellekKaydet(ta, mem) {
  try {
    await api(`/projects/${cur}/memory`, { method: 'PUT', body: JSON.stringify({ text: ta.value, expect: mem.text }) })
    open_(cur)
    say('Bellek kaydedildi')
  } catch (e) {
    say(e.status === 409 ? 'Bellek arada değişmiş — yazdıkların duruyor, kopyalayıp yeniden aç.' : 'Olmadı: ' + e.message)
  }
}

// Ekleme satiri da markdown biliyor: Ctrl+B/I/K ve panodan resim yapistirma.
function ekleKutusu() {
  const i = kutu({ name: 'text', placeholder: 'Yeni satır… (**kalın**, Ctrl+V ile resim, çok satırlı yapıştırma olduğu gibi girer)', required: true },
    (ta) => ta.form.requestSubmit())
  MD.bind(i, post, say)
  return i
}

const kopyala = (it) =>
  el('button', { className: 'cp', textContent: '⧉', title: 'Satırı kopyala', onclick: () => navigator.clipboard.writeText(it.text).then(() => say('Kopyalandı'), () => say('Kopyalanamadı')) })

// Markdown satir icinde gosterilir (kalin, kod, bag, resim); duzenlerken ham metin.
const yazi = (it, props) => {
  const s = el('span', { className: 'txt', ...props })
  s.append(MD.render(it.text))
  return s
}

function row(it, changed, ilkBaslik) {
  const ln = el('span', { className: 'ln', ariaHidden: 'true', textContent: String(it.i + 1) })
  const mark = (c) => c + (changed ? ' changed' : '') + (hi === it.text ? ' hit' : '')

  if (it.kind === 'heading') // ilk baslik zaten sayfanin basligi
    return ilkBaslik ? null : el('li', { className: mark('head h' + it.level) },
      ln, el('span', { className: 'box', ariaHidden: 'true', textContent: '#'.repeat(it.level) }), yazi(it), kopyala(it))
  // Girintili satir (ic ice madde) ekranda da girintili dursun.
  const gir = (li) => (it.indent ? (li.style.setProperty('--ind', Math.min(it.indent, 16)), li) : li)

  if (it.kind === 'text')
    return it.text.trim() ? gir(el('li', { className: mark('note') }, ln, yazi(it), el('span', { className: 'ops' }, kopyala(it)))) : null

  const id = 'l' + it.i
  const tick = el('input', { type: 'checkbox', className: 'tick', id, checked: it.done, ariaLabel: it.text, onchange: () => patch(it, { done: tick.checked }) })
  // Metin secerken duzenlemeye girmesin: parcayi kopyalamak icin secip birakmak yetsin.
  const txt = yazi(it, { onclick: () => getSelection().isCollapsed && edit(it, txt) })
  return gir(el('li', { className: mark('task' + (it.done ? ' done' : '')) },
    ln, tick,
    el('label', { className: 'box', htmlFor: id, ariaHidden: 'true', textContent: it.done ? '[x]' : '[ ]' }),
    txt,
    el('span', { className: 'ops' },
      el('button', { className: 'mv', textContent: '↑', title: 'Yukarı taşı', onclick: () => patch(it, { move: 'up' }) }),
      el('button', { className: 'mv', textContent: '↓', title: 'Aşağı taşı', onclick: () => patch(it, { move: 'down' }) }),
      el('button', { className: 'mv', textContent: '→', title: 'Başka projeye taşı', onclick: (e) => tasi(it, e.target) }),
      kopyala(it),
      el('button', { className: 'x', textContent: '×', title: 'Satırı sil', onclick: () => del(it) }))))
}

// Satiri baska projeye tasi: dugmenin yerine proje listesi acilir.
async function tasi(it, dugme) {
  const list = await api('/projects').catch(() => [])
  const sec = el('select', { className: 'tasi' },
    el('option', { value: '', textContent: '→ proje' }),
    ...list.filter((p) => p.slug !== cur).map((p) => el('option', { value: p.slug, textContent: p.title })))
  sec.onchange = () => {
    if (!sec.value) return open_(cur)
    const ad = sec.options[sec.selectedIndex].textContent
    patch(it, { to: sec.value }).then(() => say(`→ ${ad}`))
  }
  sec.onblur = () => open_(cur)
  dugme.replaceWith(sec)
  sec.focus()
}

const patch = (it, body) =>
  api(`/projects/${cur}/tasks/${it.i}`, { method: 'PATCH', body: JSON.stringify({ expect: it.text, ...body }) }).then(() => open_(cur), oops)

const del = (it) => {
  if (!confirm(`Silinsin mi?\n\n${it.text}`)) return
  api(`/projects/${cur}/tasks/${it.i}?expect=${encodeURIComponent(it.text)}`, { method: 'DELETE' }).then(() => open_(cur), oops)
}

function edit(it, node) {
  const inp = kutu({ className: 'txt edit', value: it.text }, () => inp.blur())
  node.replaceWith(inp)
  buyut(inp)
  inp.focus()
  inp.select()
  inp.onblur = () => (inp.value.trim() && inp.value !== it.text ? patch(it, { text: inp.value }) : open_(cur))
  inp.addEventListener('keydown', (e) => { if (e.key === 'Escape') { inp.onblur = null; open_(cur) } })
}

async function addTask(e) {
  e.preventDefault()
  const f = e.target.elements
  const kind = f.kind.value
  await api(`/projects/${cur}/tasks`, { method: 'POST', body: JSON.stringify({ text: f.text.value, kind }) }).catch(oops)
  f.text.value = ''
  await open_(cur)
  const next = document.querySelector('.add')
  if (next) { next.elements.kind.value = kind; next.elements.text.focus() }
}

function renameProject(p, node) {
  const inp = el('input', { className: 'edit', value: p.title })
  node.replaceWith(inp)
  inp.focus()
  inp.select()
  const done = async () => {
    inp.onblur = null
    if (!inp.value.trim() || inp.value === p.title) return open_(cur)
    const { slug } = await api('/projects/' + p.slug, { method: 'PATCH', body: JSON.stringify({ title: inp.value }) }).catch((e) => (oops(e), {}))
    if (!slug) return
    if (slug === cur) open_(cur)
    else location.hash = slug // dosya adi da degisti
  }
  inp.onblur = done
  inp.onkeydown = (e) => {
    if (e.key === 'Enter') inp.blur()
    if (e.key === 'Escape') { inp.onblur = null; open_(cur) }
  }
}

function trashProject(slug) {
  if (!confirm(`"${slug}" çöpe taşınsın mı? (notes/.trash içine gider, silinmez)`)) return
  api('/projects/' + slug, { method: 'DELETE' }).then(() => {
    cur = null
    location.hash = ''
    $('main').replaceChildren(...welcome())
    loadProjects()
    say(`${slug} → notes/.trash`)
  }, oops)
}

$('newp').onsubmit = async (e) => {
  e.preventDefault()
  const i = e.target.elements.title
  const { slug } = await api('/projects', { method: 'POST', body: JSON.stringify({ title: i.value }) }).catch((err) => (say('Olmadı: ' + err.message), {}))
  i.value = ''
  if (slug) location.hash = slug
}

/* --- bitmisleri gizle --- */
const hidden = () => localStorage.notixHideDone === '1'
function toggleDone() {
  localStorage.notixHideDone = hidden() ? '0' : '1'
  document.body.classList.toggle('hide-done', hidden())
  open_(cur)
}
document.body.classList.toggle('hide-done', hidden())

/* --- arama --- */
const find = $('find')
const finding = () => find.value.trim().length > 1
let findT
find.oninput = () => { clearTimeout(findT); findT = setTimeout(runFind, 150) }
find.onkeydown = (e) => { if (e.key === 'Escape') { find.value = ''; runFind() } }

async function runFind() {
  if (!finding()) return cur ? open_(cur) : $('main').replaceChildren(...welcome())
  const hits = await api('/search?q=' + encodeURIComponent(find.value.trim())).catch(() => [])
  $('main').replaceChildren(
    el('header', {}, el('h2', { textContent: `“${find.value.trim()}”` }), el('span', { className: 'meta', textContent: `${hits.length} satır` })),
    hits.length
      ? el('ul', { className: 'hits' }, hits.map((h) =>
        el('li', { onclick: () => go(h.slug, h.text) },
          el('span', { className: 'where', textContent: h.project }),
          el('span', { className: 'box', ariaHidden: 'true', textContent: h.kind === 'task' ? (h.done ? '[x]' : '[ ]') : '¶' }),
          el('span', { className: 'txt' + (h.done ? ' off' : ''), textContent: h.text }))))
      : el('p', { className: 'empty', textContent: 'Eşleşen satır yok.' }))
}

function go(slug, text) {
  find.value = ''
  hi = text
  if (location.hash.slice(1) === slug) open_(slug)
  else location.hash = slug
}

/* --- belge modu: dosyanin tamami markdown olarak. Not, bellek, resim, baslik --- */
let editing = false
const post = (p, body) => api(p, { method: 'POST', body: JSON.stringify(body) })

async function openDoc() {
  const proje = cur
  const { text } = await api(`/projects/${proje}/raw`).catch((e) => (say('Olmadı: ' + e.message), {}))
  if (text == null) return
  editing = true

  const ta = el('textarea', { className: 'doc', value: text, spellcheck: false })
  MD.bind(ta, post, say)
  const dosya = el('input', { type: 'file', accept: 'image/*', hidden: true, onchange: async (e) => {
    const f = e.target.files[0]; e.target.value = ''
    if (f) try { MD.wrap(ta, await MD.upload(f, post), '') } catch (err) { say('Resim olmadı: ' + err.message) }
  } })
  const dugme = (etiket, ipucu, is) => el('button', { className: 'link', textContent: etiket, title: ipucu, onclick: is })

  $('main').replaceChildren(
    el('header', {},
      el('span', { className: 'hash', ariaHidden: 'true', textContent: '#' }),
      el('h2', { textContent: proje }),
      el('span', { className: 'meta', textContent: 'markdown · tüm dosya' }),
      el('button', { className: 'link', textContent: 'Vazgeç', onclick: () => kapat(ta, text) }),
      el('button', { className: 'link ok', textContent: 'Kaydet', onclick: () => saveDoc(ta, text) })),
    el('div', { className: 'tools' },
      dugme('H1', 'Başlık', () => MD.prefix(ta, '# ')),
      dugme('H2', 'Alt başlık', () => MD.prefix(ta, '## ')),
      dugme('B', 'Kalın (Ctrl+B)', () => MD.wrap(ta, '**')),
      dugme('I', 'Eğik (Ctrl+I)', () => MD.wrap(ta, '*')),
      dugme('`kod`', 'Kod', () => MD.wrap(ta, '`')),
      dugme('•', 'Madde', () => MD.prefix(ta, '- ')),
      dugme('[ ]', 'Yapılacak', () => MD.prefix(ta, '- [ ] ')),
      dugme('bağ', 'Bağlantı (Ctrl+K)', () => MD.wrap(ta, '[', '](https://)')),
      dugme('resim', 'Resim ekle (panodan da yapıştırabilirsin)', () => dosya.click()),
      dosya),
    ta,
    el('p', { className: 'hint', textContent: 'Serbest metin ve başlıklar dosyada olduğu gibi durur — projenin belleği burada tutulabilir. AI ajanları da aynı dosyayı okuyor.' }))
  ta.focus()
}

const kapat = (ta, eski) => {
  if (ta.value !== eski && !confirm('Kaydedilmemiş değişiklikler var, çıkılsın mı?')) return
  editing = false
  open_(cur)
}

async function saveDoc(ta, eski) {
  try {
    await api(`/projects/${cur}/raw`, { method: 'PUT', body: JSON.stringify({ text: ta.value, expect: eski }) })
    editing = false
    open_(cur)
    say('Belge kaydedildi')
  } catch (e) {
    // 409'da editoru kapatma: kullanicinin yazdigi kaybolmasin.
    say(e.status === 409 ? 'Dosya arada değişmiş — yazdıkların duruyor, kopyalayıp yeniden aç.' : 'Olmadı: ' + e.message)
  }
}

/* --- cop kutusu --- */
const when = (ms) => new Date(ms).toLocaleString('tr-TR', { dateStyle: 'short', timeStyle: 'short' })

async function showTrash() {
  cur = null // poll bu ekrani ezmesin
  location.hash = ''
  const bin = await api('/trash').catch((e) => (say('Olmadı: ' + e.message), []))
  $('main').replaceChildren(
    el('header', {},
      el('span', { className: 'hash', ariaHidden: 'true', textContent: '#' }),
      el('h2', { textContent: 'Çöp kutusu' }),
      el('span', { className: 'meta', textContent: `notes/.trash · ${bin.length} dosya` })),
    bin.length
      ? el('ul', { className: 'bin' }, bin.map((b) =>
        el('li', {},
          el('span', { className: 'when', textContent: when(b.at) }),
          el('span', { className: 'txt' },
            el('span', { className: 'name', textContent: b.title }),
            el('span', { className: 'file', textContent: `${b.file} · ${b.open} açık, ${b.done} bitti` })),
          el('button', { className: 'link', textContent: 'Geri al', onclick: () => restore(b) }),
          el('button', { className: 'link warn', textContent: 'Kalıcı sil', onclick: () => purge(b) }))))
      : el('p', { className: 'empty', textContent: 'Çöp kutusu boş.' }),
    el('p', { className: 'hint', textContent: 'Silinen projeler ve içe aktarmada üzerine yazılan dosyalar burada durur. Kalıcı silme geri alınamaz.' }))
  loadProjects()
}

async function restore(b) {
  const { slug } = await api(`/trash/${encodeURIComponent(b.file)}/restore`, { method: 'POST' }).catch((e) => (say('Olmadı: ' + e.message), {}))
  if (!slug) return
  say(slug === b.file.replace(/-\d+\.md$/, '') ? `${slug} geri alındı` : `Aynı adda proje vardı, ${slug} olarak geri alındı`)
  showTrash()
}

async function purge(b) {
  if (!confirm(`"${b.title}" kalıcı olarak silinsin mi?\n\n${b.file}\nBu geri alınamaz.`)) return
  await api(`/trash/${encodeURIComponent(b.file)}`, { method: 'DELETE' }).catch((e) => say('Olmadı: ' + e.message))
  showTrash()
}

$('bin').onclick = showTrash

/* --- bugun: tum projelerden tarihi gelmis acik isler (@YYYY-MM-DD) --- */
async function showToday() {
  cur = null // poll bu ekrani ezmesin
  location.hash = ''
  const g = bugun()
  const hits = await api('/due?d=' + g).catch((e) => (say('Olmadı: ' + e.message), []))
  $('main').replaceChildren(
    el('header', {},
      el('span', { className: 'hash', ariaHidden: 'true', textContent: '#' }),
      el('h2', { textContent: 'Bugün' }),
      el('span', { className: 'meta', textContent: `${g} · ${hits.length} iş` })),
    hits.length
      ? el('ul', { className: 'hits' }, hits.map((h) =>
        el('li', { onclick: () => go(h.slug, h.text) },
          el('span', { className: 'where', textContent: h.project }),
          el('span', { className: 'box', ariaHidden: 'true', textContent: '[ ]' }),
          el('span', { className: 'txt' }, MD.render(h.text)))))
      : el('p', { className: 'empty', textContent: 'Tarihi gelmiş iş yok.' }),
    el('p', { className: 'hint', textContent: 'Bir işe tarih vermek için satıra @2026-09-15 yaz. Tarihi bugün veya geçmiş olan açık işler burada toplanır — AI ajanları da aynı kuralı kullanabilir.' }))
  loadProjects()
}

$('today').onclick = showToday

/* --- disa/ice aktarma: veriyi baska makineye tasimak icin --- */
$('exp').onclick = async () => {
  const bundle = await api('/export').catch((e) => (say('Olmadı: ' + e.message), null))
  if (!bundle) return
  const url = URL.createObjectURL(new Blob([JSON.stringify(bundle, null, 2)], { type: 'application/json' }))
  const a = el('a', { href: url, download: `notix-${new Date().toISOString().slice(0, 10)}.json` })
  document.body.append(a)
  a.click()
  a.remove()
  URL.revokeObjectURL(url)
  say(`${Object.keys(bundle.files).length} proje dışa aktarıldı`)
}

$('imp').onclick = () => $('file').click()
$('file').onchange = async (e) => {
  const files = {}
  for (const f of e.target.files) {
    const text = await f.text()
    if (f.name.endsWith('.json')) Object.assign(files, JSON.parse(text).files || JSON.parse(text))
    else files[f.name.replace(/\.md$/i, '')] = text
  }
  e.target.value = ''
  const n = Object.keys(files).length
  if (!n || !confirm(`${n} proje içe aktarılacak.\nAynı adlı dosya varsa eskisi notes/.trash içine kopyalanır.`)) return
  const r = await api('/import', { method: 'POST', body: JSON.stringify({ files }) }).catch((err) => (say('Olmadı: ' + err.message), null))
  if (!r) return
  say(`${r.written} proje alındı${r.replaced.length ? ` · ${r.replaced.length} tanesinin eskisi .trash'e kopyalandı` : ''}`)
  loadProjects()
  if (cur) open_(cur)
}

/* --- kisayollar --- */
const LABEL = { quickAdd: 'Hızlı ekleme (uygulama açıkken her yerde)', find: 'Ara', add: 'Yeni satır', hideDone: 'Bitmişleri gizle/göster', help: 'Bu pencere' }
const OPT = { openAtLogin: 'Bilgisayar açılınca başlat', autoUpdate: 'Açılışta güncelleme denetle', backup: 'Otomatik yedek (notes klasöründe git commit)' }
let keys = {}
let desktop = false // Electron içinde mi (masaüstü seçenekleri ancak o zaman anlamlı)
let update = null // /api/update sonucu, yeni sürüm varsa
let cap = null // hangi kisayol yakalama modunda

// Tarayici olayindan Electron accelerator metni: yakalama da eslestirme de bunu kullanir.
function accel(e) {
  const k = e.key
  if (['Control', 'Alt', 'Shift', 'Meta', 'Dead'].includes(k)) return ''
  const named = k.length > 1 || /[a-z0-9]/i.test(k)
  return [e.ctrlKey && 'Control', e.altKey && 'Alt', e.shiftKey && named && 'Shift', e.metaKey && 'Super',
    k.length === 1 ? k.toUpperCase() : k].filter(Boolean).join('+')
}

const run = {
  find: () => find.focus(),
  add: () => document.querySelector('.add textarea')?.focus(),
  hideDone: () => toggleDone(),
  help: () => showKeys(),
}

addEventListener('keydown', (e) => {
  if (cap) {
    e.preventDefault()
    if (e.key === 'Escape') { cap = null; return showKeys() }
    const a = accel(e)
    if (a) saveKey(cap, a)
    return
  }
  const a = accel(e)
  if (!a) return
  const typing = /^(INPUT|SELECT|TEXTAREA)$/.test(e.target.tagName)
  if (typing && !e.ctrlKey && !e.altKey && !e.metaKey) return // yazarken tek harfli kisayol calismasin
  for (const [name, k] of Object.entries(keys))
    if (k === a && run[name]) { e.preventDefault(); return run[name]() }
})

function applyKeys(next) {
  keys = { version: keys.version, ...next }
  find.placeholder = `Ara… (${keys.find})`
  $('help').title = `Kısayollar (${keys.help})`
  if (!cur) $('main').replaceChildren(...welcome()) // acilis ekranindaki kısayol yazisi guncel kalsin
}

function showKeys() {
  const d = $('keys')
  d.replaceChildren(
    el('h3', { textContent: 'Kısayollar' }),
    el('table', {}, el('tbody', {}, ...Object.keys(LABEL).map((name) =>
      el('tr', { className: cap === name ? 'capturing' : '' },
        el('td', { textContent: LABEL[name] }),
        el('td', {}, el('code', { textContent: cap === name ? 'tuşlara bas…' : keys[name] || '—' })),
        el('td', {}, el('button', { className: 'link', textContent: cap === name ? 'vazgeç' : 'değiştir', onclick: () => { cap = cap === name ? null : name; showKeys() } })))))),
    el('p', { className: 'hint', textContent: 'Satıra tıkla → düzenle. Enter kaydeder, Esc vazgeçer. Değiştirirken Esc yakalamayı bırakır.' }),
    ...(desktop ? [
      el('h3', { textContent: 'Uygulama' }),
      ...Object.entries(OPT).map(([name, text]) => el('label', { className: 'opt' },
        el('input', { type: 'checkbox', checked: !!keys[name], onchange: (e) => saveOpt(name, e.target.checked) }), ' ' + text)),
      el('p', { className: 'hint' }, `Sürüm ${keys.version || '?'}`,
        ...(update?.available ? [' · ', el('a', { href: update.url, target: '_blank', textContent: `yeni sürüm ${update.latest} — indir` })] : [])),
    ] : []),
    el('form', { method: 'dialog' }, el('button', { textContent: 'Kapat' })))
  if (!d.open) d.showModal()
}

async function saveKey(name, a) {
  const r = await api('/config', { method: 'PUT', body: JSON.stringify({ ...keys, [name]: a }) }).catch((e) => (say('Olmadı: ' + e.message), null))
  cap = null
  if (!r) return showKeys()
  applyKeys(r.keys)
  showKeys()
  if (name === 'quickAdd' && r.bound === false) say(`${a} başka bir uygulamada, hızlı ekleme çalışmayacak`)
}

async function saveOpt(name, on) {
  const r = await api('/config', { method: 'PUT', body: JSON.stringify({ ...keys, [name]: on }) }).catch((e) => (say('Olmadı: ' + e.message), null))
  if (r) applyKeys(r.keys)
  showKeys()
}

// ponytail: sadece haber verir; indirip kurma electron-updater ile sonra.
async function checkUpdate() {
  update = await api('/update').catch(() => null)
  if (update?.available) say(`Yeni sürüm ${update.latest} var · ⌨ penceresinden indir`)
}

$('help').onclick = showKeys
$('keys').addEventListener('close', () => { cap = null })

const welcome = () => [
  el('p', { className: 'empty', textContent: 'Soldan bir proje seç ya da yukarıdan yeni bir tane aç.' }),
  el('p', { className: 'hint' }, el('kbd', { textContent: keys.quickAdd || 'Control+Alt+N' }),
    ' — nerede olursan ol, Inbox\'a bir satır ekler.'),
]

// Sayfa file:// baglantisi acamiyor; klasoru Electron aciyor. Tarayicida acilmaz,
// o zaman yol panoya kopyalanir.
const openDir = (dir) => api('/open', { method: 'POST' })
  .then((r) => r.ok || navigator.clipboard.writeText(dir).then(() => say('Klasör açılamadı, yol kopyalandı'), () => say(dir)))
  .catch((e) => say('Olmadı: ' + e.message))

addEventListener('hashchange', () => open_(decodeURIComponent(location.hash.slice(1))))

// AI veya editör dosyayı değiştirirse ekran kendiliğinden tazelensin.
setInterval(() => {
  if (document.hidden || editing || /^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement?.tagName) || $('keys').open) return
  cur ? open_(cur, true).catch(() => {}) : loadProjects().catch(() => {})
}, 5000)

api('/info')
  .then(({ dir, version }) => { keys.version = version; $('where').replaceChildren(
    el('button', { className: 'link', title: 'Klasörü aç', textContent: dir, onclick: () => openDir(dir) })) })
  .catch(() => {})

api('/config').then((c) => {
  desktop = c.desktop
  applyKeys({ ...keys, ...c.keys })
  if (desktop && c.keys.autoUpdate) checkUpdate()
}).catch(() => {})

$('main').replaceChildren(...welcome())
loadProjects()
  .then(() => open_(decodeURIComponent(location.hash.slice(1))))
  .catch((e) => $('main').replaceChildren(el('p', { className: 'empty', textContent: 'Bağlanamadı: ' + e.message })))
