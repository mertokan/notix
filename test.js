// node --test
const test = require('node:test')
const assert = require('node:assert')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')

process.env.NOTIX_DIR = fs.mkdtempSync(path.join(os.tmpdir(), 'notix-'))
const store = require('./store')

test('yazilan her sey korunur, sadece hedef satir degisir', () => {
  const slug = store.create('Servix Web')
  assert.equal(slug, 'servix-web')

  fs.writeFileSync(path.join(store.DIR, 'servix-web.md'), [
    '# Servix Web',
    '',
    'AI notu: burasi elle yazilmis serbest metin, silinmemeli.',
    '- [ ] gateway timeout ayarla',
    '  - [x] alt gorev',
    '',
  ].join('\n'))

  const p = store.parse(slug)
  assert.equal(p.title, 'Servix Web')
  assert.deepEqual(p.items.filter((x) => x.kind === 'task').map((x) => [x.i, x.done, x.text]), [
    [3, false, 'gateway timeout ayarla'],
    [4, true, 'alt gorev'],
  ])

  store.updateTask(slug, 3, { expect: 'gateway timeout ayarla', done: true })
  store.addTask(slug, 'yeni gorev')
  const after = fs.readFileSync(path.join(store.DIR, 'servix-web.md'), 'utf8')
  assert.match(after, /AI notu: burasi elle yazilmis serbest metin, silinmemeli\./)
  assert.match(after, /- \[x\] gateway timeout ayarla/)
  assert.match(after, /^ {2}- \[x\] alt gorev$/m) // girinti korunur
  assert.match(after, /- \[ \] yeni gorev/)
})

test('arada degisen satiri ezmez (409)', () => {
  const slug = store.create('Yaris')
  store.addTask(slug, 'ilk')
  const i = store.parse(slug).items.find((x) => x.kind === 'task').i
  store.updateTask(slug, i, { text: 'AI bunu degistirdi' }) // baskasi yazdi
  assert.throws(() => store.updateTask(slug, i, { expect: 'ilk', done: true }), { code: 409 })
})

test('hizli ekleme: Inbox ikinci kez olusturulunca sifirlanmaz', () => {
  const slug = store.create('Inbox') // kutu her acilista create cagiriyor
  store.addTask(slug, 'ilk fikir')
  assert.equal(store.create('Inbox'), slug)
  store.addTask(slug, 'ikinci fikir')
  assert.deepEqual(
    store.parse(slug).items.filter((x) => x.kind === 'task').map((x) => x.text),
    ['ilk fikir', 'ikinci fikir'])
})

test('turkce harfler slugda kaybolmaz', () => {
  assert.equal(store.create('Ev İşleri'), 'ev-isleri')
  assert.equal(store.slugify('Çöp Güncesi ıI'), 'cop-guncesi-ii')
})

test('slug disari cikamaz', () => {
  assert.throws(() => store.parse('../../etc/passwd'), { code: 400 })
})

test('ad degistirme: sadece baslik satiri ve dosya adi degisir', () => {
  const slug = store.create('Eski Ad')
  store.addTask(slug, 'duran is')
  fs.appendFileSync(path.join(store.DIR, slug + '.md'), 'elle yazilmis not\n')

  const next = store.rename(slug, 'Yeni Ad')
  assert.equal(next, 'yeni-ad')
  const after = fs.readFileSync(path.join(store.DIR, 'yeni-ad.md'), 'utf8')
  assert.match(after, /^# Yeni Ad$/m)
  assert.match(after, /- \[ \] duran is/)
  assert.match(after, /elle yazilmis not/)
  assert.equal(fs.existsSync(path.join(store.DIR, 'eski-ad.md')), false)

  store.create('Cakisan')
  assert.throws(() => store.rename('yeni-ad', 'Cakisan'), { code: 409 })
})

test('alt gorev girintili, serbest not isaretsiz eklenir', () => {
  const slug = store.create('Satir Turleri')
  store.addTask(slug, 'ana is')
  store.addTask(slug, 'alt is', 'sub')
  store.addTask(slug, 'sadece not', 'note')
  const items = store.parse(slug).items.filter((x) => x.kind !== 'heading' && x.text.trim())
  assert.deepEqual(items.map((x) => [x.kind, x.text]), [['task', 'ana is'], ['task', 'alt is'], ['text', 'sadece not']])
  assert.match(fs.readFileSync(path.join(store.DIR, slug + '.md'), 'utf8'), /^ {2}- \[ \] alt is$/m)
})

test('arama tum projelerde, buyuk/kucuk harf farketmez', () => {
  store.importFile('arama-1', '# Arama 1\n- [ ] Kritik Kayit\n')
  store.importFile('arama-2', '# Arama 2\n- [x] kritik olmayan\n')
  const hits = store.search('KRITIK')
  assert.deepEqual(hits.map((h) => h.slug).sort(), ['arama-1', 'arama-2'])
  assert.equal(hits.find((h) => h.slug === 'arama-2').done, true)
  assert.equal(store.search('k').length, 0) // tek harf aranmaz
})

test('ice aktarma eskisini ezmeden once .trash e kopyalar', () => {
  store.importFile('tasinan', '# Tasinan\n- [ ] ilk surum\n')
  const r = store.importFile('tasinan', '# Tasinan\n- [ ] ikinci surum\n')
  assert.equal(r.replaced, true)
  assert.match(fs.readFileSync(path.join(store.DIR, 'tasinan.md'), 'utf8'), /ikinci surum/)
  const yedek = fs.readdirSync(path.join(store.DIR, '.trash')).filter((f) => f.startsWith('tasinan-'))
  assert.equal(yedek.length, 1)
  assert.match(fs.readFileSync(path.join(store.DIR, '.trash', yedek[0]), 'utf8'), /ilk surum/)
  assert.equal(store.exportAll()['tasinan'], '# Tasinan\n- [ ] ikinci surum\n')
  // disaridan gelen ad temizlenir: klasor disina cikamaz, hicbir gecerli harf yoksa reddedilir
  assert.equal(store.importFile('../kacak', 'x').slug, 'kacak')
  assert.throws(() => store.importFile('../..', 'x'), { code: 400 })
})

test('bellek: bolum yoksa acilir, varsa sadece o araliga dokunur', () => {
  const slug = store.create('Bellekli')
  store.addTask(slug, 'duran is')
  fs.appendFileSync(path.join(store.DIR, slug + '.md'), '## Notlar\n\nelle yazilmis\n')

  store.setMemory(slug, 'Prod veritabani `x`.\nKarar: auth NextAuth.')
  const m = store.memory(slug)
  assert.match(m.text, /Prod veritabani/)
  const dosya = fs.readFileSync(path.join(store.DIR, slug + '.md'), 'utf8')
  assert.match(dosya, /^# Bellekli\n/)          // baslik yerinde
  assert.match(dosya, /## Bellek/)
  assert.match(dosya, /- \[ \] duran is/)        // isler duruyor
  assert.match(dosya, /## Notlar[\s\S]*elle yazilmis/) // diger bolum bozulmadi

  // bellek en altta: yeni is onun ustune girer, bellege karismaz
  store.addTask(slug, 'sonradan eklenen is')
  assert.equal(store.memory(slug).text.includes('sonradan eklenen is'), false)
  assert.equal(store.parse(slug).items.some((x) => x.kind === 'task' && x.text === 'sonradan eklenen is'), true)

  store.setMemory(slug, 'Tek satir', store.memory(slug).text) // guncelleme
  assert.equal(store.memory(slug).text, 'Tek satir')
  assert.match(fs.readFileSync(path.join(store.DIR, slug + '.md'), 'utf8'), /## Notlar/)

  assert.throws(() => store.setMemory(slug, 'baska', 'eski hali'), { code: 409 })

  store.setMemory(slug, '', 'Tek satir')         // bosaltinca bolum kalkar
  assert.equal(store.memory(slug).from, -1)
  assert.doesNotMatch(fs.readFileSync(path.join(store.DIR, slug + '.md'), 'utf8'), /## Bellek/)
})

test('belge modu: tum dosya yazilir, arada degistiyse 409', () => {
  const slug = store.create('Bellek')
  store.addTask(slug, 'ilk is')
  const eski = store.read(slug)

  store.write(slug, '# Bellek\n\nUzun not.\n\n![ekran](media/a-1.png)\n\n## Kararlar\n\n- [ ] ilk is\n', eski)
  const p = store.parse(slug)
  assert.equal(p.items.find((x) => x.kind === 'heading' && x.level === 2).text, 'Kararlar')
  assert.equal(p.items.some((x) => x.text === 'Uzun not.'), true)
  assert.equal(p.items.filter((x) => x.kind === 'task').length, 1)

  // arada baskasi yazdi: eski icerikle kaydetmeye calisinca 409
  assert.throws(() => store.write(slug, '# Bellek\n', eski), { code: 409 })
})

test('resim: sadece bilinen uzanti, klasor disina cikamaz', () => {
  const yol = store.saveMedia('Ekran Görüntüsü.png', Buffer.from('deneme').toString('base64'))
  assert.match(yol, /^media\/ekran-goruntusu-\d+\.png$/)
  assert.equal(fs.readFileSync(path.join(store.DIR, yol), 'utf8'), 'deneme')
  assert.equal(store.mediaFile(yol.slice(6)).type, 'image/png')
  assert.throws(() => store.saveMedia('kotu.exe', 'AA=='), { code: 400 })
  assert.throws(() => store.mediaFile('../../store.js'), { code: 400 })
})

test('cop kutusu: geri alma, ad cakismasi ve kalici silme', () => {
  const slug = store.create('Copluk')
  store.addTask(slug, 'geri gelecek is')
  store.remove(slug)

  const bin = store.trash().filter((b) => b.file.startsWith('copluk-'))
  assert.equal(bin.length, 1)
  assert.equal(bin[0].title, 'Copluk')
  assert.equal(bin[0].open, 1)

  assert.equal(store.restore(bin[0].file), 'copluk') // eski adina doner
  assert.match(fs.readFileSync(path.join(store.DIR, 'copluk.md'), 'utf8'), /geri gelecek is/)
  assert.equal(store.trash().some((b) => b.file === bin[0].file), false)

  store.remove('copluk')
  store.create('Copluk') // ad bosalinca yenisi acildi
  const yine = store.trash().find((b) => b.file.startsWith('copluk-'))
  assert.equal(store.restore(yine.file), 'copluk-2') // ustune yazmaz

  store.remove('copluk-2')
  const silinecek = store.trash().find((b) => b.file.startsWith('copluk-2-'))
  store.purge(silinecek.file)
  assert.equal(fs.existsSync(path.join(store.DIR, '.trash', silinecek.file)), false)
  assert.throws(() => store.purge('../../store.js'), { code: 400 })
})

test('proje silme = .trash e tasima', () => {
  const slug = store.create('Gecici')
  store.remove(slug)
  assert.equal(store.list().some((p) => p.slug === slug), false)
  assert.equal(fs.readdirSync(path.join(store.DIR, '.trash')).filter((f) => f.startsWith(slug + '-')).length, 1)
})

test('surum karsilastirma: sadece daha yeni olan guncelleme sayilir', () => {
  process.env.NOTIX_HOME = store.DIR // token/ayar repo'ya degil gecici dizine yazilsin
  const { newer } = require('./server')
  assert.equal(newer('0.2.0', '0.1.0'), true)
  assert.equal(newer('0.1.0', '0.1.0'), false)
  assert.equal(newer('0.1.10', '0.1.9'), true)
  assert.equal(newer('1.0', '0.9.9'), true)
  assert.equal(newer('0.0.9', '0.1.0'), false)
})
