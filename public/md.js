// Markdown: satir icinde gosterim + yazma kisayollari. Iki sayfa da kullaniyor
// (ana pencere ve hizli ekleme kutusu), o yuzden ayri dosya. Build adimi yok.
const MD = (() => {
  let token = ''
  // Kullanici metni hicbir zaman innerHTML ile basilmaz; hepsi DOM dugumu.
  const PARCA = /(!\[[^\]]*\]\([^)\s]+\)|\[[^\]]+\]\([^)\s]+\)|\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`|@\d{4}-\d{2}-\d{2})/g
  // Yerel gun: toISOString UTC verir, gece yarisi civari bir gun kayardi.
  const bugun = () => { const d = new Date(); return new Date(d - d.getTimezoneOffset() * 6e4).toISOString().slice(0, 10) }
  const guvenli = (u) => /^https?:\/\//i.test(u) || /^media\/[a-z0-9][a-z0-9-]*\.[a-z]+$/i.test(u)
  const src = (u) => (u.startsWith('media/') ? `/${u}?t=${encodeURIComponent(token)}` : u)

  function dugum(tok) {
    let m
    if ((m = /^!\[([^\]]*)\]\(([^)\s]+)\)$/.exec(tok))) {
      if (!guvenli(m[2])) return tok // javascript: vb. — duz metin kalsin
      const img = document.createElement('img')
      img.src = src(m[2]); img.alt = m[1]; img.loading = 'lazy'
      return img
    }
    if ((m = /^\[([^\]]+)\]\(([^)\s]+)\)$/.exec(tok))) {
      if (!guvenli(m[2])) return tok
      const a = document.createElement('a')
      a.href = src(m[2]); a.target = '_blank'; a.rel = 'noreferrer'; a.textContent = m[1]
      return a
    }
    // @2026-09-15 → tarih rozeti. Gecmis kirmizi, bugun patina, ilerisi soluk.
    if ((m = /^@(\d{4}-\d{2}-\d{2})$/.exec(tok))) {
      const g = bugun()
      return Object.assign(document.createElement('span'), {
        className: 'due' + (m[1] < g ? ' gecti' : m[1] === g ? ' bugun' : ''),
        textContent: m[1],
      })
    }
    if ((m = /^\*\*(.+)\*\*$/.exec(tok))) return Object.assign(document.createElement('strong'), { textContent: m[1] })
    if ((m = /^\*(.+)\*$/.exec(tok))) return Object.assign(document.createElement('em'), { textContent: m[1] })
    if ((m = /^`(.+)`$/.exec(tok))) return Object.assign(document.createElement('code'), { textContent: m[1] })
    return tok
  }

  function render(text) {
    const frag = document.createDocumentFragment()
    let son = 0
    for (const m of String(text).matchAll(PARCA)) {
      if (m.index > son) frag.append(text.slice(son, m.index))
      frag.append(dugum(m[0]))
      son = m.index + m[0].length
    }
    if (son < text.length) frag.append(text.slice(son))
    return frag
  }

  // Secimi isaretle: secim yoksa imleci isaretin ortasina birakir.
  function wrap(el, once, sonra = once) {
    const { selectionStart: a, selectionEnd: b, value: v } = el
    el.value = v.slice(0, a) + once + v.slice(a, b) + sonra + v.slice(b)
    el.selectionStart = a + once.length
    el.selectionEnd = b + once.length
    el.focus()
    el.dispatchEvent(new Event('input', { bubbles: true }))
  }

  // Imlecin bulundugu satirin basina ekler/kaldirir (baslik, madde).
  function prefix(el, isaret) {
    const v = el.value
    const bas = v.lastIndexOf('\n', el.selectionStart - 1) + 1
    const vardi = v.slice(bas).startsWith(isaret)
    el.value = v.slice(0, bas) + (vardi ? v.slice(bas + isaret.length) : isaret + v.slice(bas))
    const k = vardi ? -isaret.length : isaret.length
    el.selectionStart = el.selectionEnd = Math.max(bas, el.selectionStart + k)
    el.focus()
    el.dispatchEvent(new Event('input', { bubbles: true }))
  }

  const b64 = (dosya) => new Promise((ok, hata) => {
    const r = new FileReader()
    r.onload = () => ok(String(r.result).split(',')[1])
    r.onerror = hata
    r.readAsDataURL(dosya)
  })

  // Panodan/dosyadan resmi notes/media/ altina yazar, markdown'ini dondurur.
  async function upload(dosya, gonder) {
    const { path } = await gonder('/media', { name: dosya.name || 'resim.png', data: await b64(dosya) })
    return `![${(dosya.name || '').replace(/\.[^.]+$/, '')}](${path})`
  }

  // Ctrl+B/I/K + resim yapistirma. gonder: (yol, govde) => Promise
  function bind(el, gonder, bildir = () => {}) {
    el.addEventListener('keydown', (e) => {
      if (!e.ctrlKey && !e.metaKey) return
      const k = e.key.toLowerCase()
      if (k === 'b') { e.preventDefault(); wrap(el, '**') }
      else if (k === 'i') { e.preventDefault(); wrap(el, '*') }
      else if (k === 'k') { e.preventDefault(); wrap(el, '[', '](https://)') }
    })
    el.addEventListener('paste', async (e) => {
      const it = [...(e.clipboardData?.items || [])].find((x) => x.type.startsWith('image/'))
      if (!it) return
      e.preventDefault()
      try { wrap(el, await upload(it.getAsFile(), gonder), '') } catch (err) { bildir('Resim olmadı: ' + err.message) }
    })
  }

  return { render, wrap, prefix, bind, upload, setToken: (t) => (token = t) }
})()
