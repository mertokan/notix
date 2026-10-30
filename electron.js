const { app, BrowserWindow, Menu, Tray, Notification, dialog, globalShortcut, nativeImage, nativeTheme, shell } = require('electron')
const path = require('node:path')
const fs = require('node:fs')

const ICON = path.join(__dirname, 'public', 'icon.png')

// Kurulu uygulamada kaynak klasoru salt okunur (app.asar): token/ayar userData'ya,
// notlar Belgeler/Notix'e. Gelistirirken (npm start) her sey repo icinde kalir.
if (app.isPackaged) process.env.NOTIX_HOME ||= app.getPath('userData')
// Ayarlardan secilen klasor: store.DIR require aninda sabitlendigi icin server'dan once okunmali.
if (!process.env.NOTIX_DIR) {
  let dir = ''
  try { dir = JSON.parse(fs.readFileSync(path.join(process.env.NOTIX_HOME || __dirname, '.notix-config.json'), 'utf8')).dir || '' } catch {}
  if (dir && fs.existsSync(dir)) process.env.NOTIX_DIR = dir
  else if (app.isPackaged) process.env.NOTIX_DIR = path.join(app.getPath('documents'), 'Notix')
}
const { start, PORT, TOKEN, config, onConfig, onWindow, onOpen, onPick } = require('./server')
const store = require('./store')

// Acilir listeler/kaydirma cubuklari Chromium'un kendi ciziminde: sayfa CSS'i
// oraya gecmiyor, tema buradan zorlanmali.
const ACIK = ['light', 'paper']
const themeOf = (t) => (ACIK.includes(t) ? 'light' : t === 'system' ? 'system' : 'dark')
// Sayfa yuklenene kadar gorunen renk: yanlis temada beyaz/siyah flas olmasin.
const bg = (main) => (nativeTheme.shouldUseDarkColors ? (main ? '#191713' : '#161922') : (main ? '#f6f4ef' : '#ffffff'))
nativeTheme.themeSource = themeOf(config().theme)

// Tepsi ve bildirim metinleri. Sayfanin kendi cevirisi public/ tarafinda.
const T = {
  tr: ['Notix', 'Hızlı ekle', 'Çık', 'iş bugün ya da gecikmiş'],
  en: ['Notix', 'Quick add', 'Quit', 'tasks due today or overdue'],
  de: ['Notix', 'Schnell hinzufügen', 'Beenden', 'Aufgaben heute fällig oder überfällig'],
  es: ['Notix', 'Añadir rápido', 'Salir', 'tareas para hoy o atrasadas'],
  fr: ['Notix', 'Ajout rapide', 'Quitter', "tâches dues aujourd'hui ou en retard"],
  pt: ['Notix', 'Adicionar rápido', 'Sair', 'tarefas para hoje ou atrasadas'],
  ru: ['Notix', 'Быстро добавить', 'Выход', 'задач на сегодня или просрочено'],
  ja: ['Notix', 'クイック追加', '終了', '件のタスクが今日期限または期限切れ'],
  zh: ['Notix', '快速添加', '退出', '项任务今天到期或已逾期'],
}
const dil = (lang) => T[lang === 'auto' || !T[lang] ? app.getLocale().split('-')[0] : lang] || T.en
// Varsayilan menu yok: Alt'a basinca gizli menu cubugu acilip (Dosya > Cikis)
// uygulama kapanabiliyordu. Kopyala/yapistir Chromium'da menusuz de calisiyor.
Menu.setApplicationMenu(null)

// Ikinci kez acilirsa port zaten dolu olurdu (pencere "baglanamadi" ile acilir);
// yeni ornek cikar, var olanin penceresini one getirir.
let main = null
if (!app.requestSingleInstanceLock()) { app.quit(); return }
function goster() {
  if (!main) return
  if (main.isMinimized()) main.restore()
  main.show()
  main.focus()
}
app.on('second-instance', goster)

start()

// ponytail: kutu her kisayolda sifirdan acilir (~150ms). Gec geldigi hissedilirse
// gizli pencere tutup show()/hide() yap.
let quick = null
function quickAdd() {
  if (quick) return quick.focus()
  quick = new BrowserWindow({
    width: 560, height: 62, frame: false, resizable: false, alwaysOnTop: true,
    skipTaskbar: true, backgroundColor: bg(false), show: false,
  })
  quick.loadURL(`http://127.0.0.1:${PORT}/quick.html?t=${TOKEN}`)
  quick.once('ready-to-show', () => quick.show())
  quick.on('blur', () => quick.close()) // ponytail: odak gidince yarim satir gider, Spotlight gibi
  quick.on('closed', () => { quick = null })
}

// Kutu proje listesini acip kapatinca yuksekligi buradan degisir (sinirli aralik).
onWindow((h) => {
  if (!quick) return false
  const b = quick.getBounds()
  quick.setBounds({ ...b, height: Math.max(62, Math.min(420, Math.round(h))) })
  return true
})

// Sol alttaki yol yazisina basinca not klasoru Gezgin'de acilir.
onOpen((dir) => (shell.openPath(dir), true))

// Klasor degisince store.DIR yeni yolla kurulsun diye yeniden baslatilir (server yolu once yazar).
onPick(async () => {
  const r = await dialog.showOpenDialog(main, { properties: ['openDirectory', 'createDirectory'] })
  if (r.canceled || !r.filePaths[0]) return null
  setTimeout(() => { app.relaunch(); app.exit(0) }, 500)
  return r.filePaths[0]
})

// Tarihi gelmis isler: gun basina bir bildirim. shortcut: bellekte tutuluyor, yeniden baslatinca bir kez daha gelir.
const yerelBugun = () => { const d = new Date(); return new Date(d - d.getTimezoneOffset() * 6e4).toISOString().slice(0, 10) }
let bildirilen = ''
function bildir() {
  const cfg = config()
  const gun = yerelBugun()
  if (!cfg.notify || bildirilen === gun || !Notification.isSupported()) return
  let isler
  try { isler = store.due(gun) } catch (e) { return console.warn('notix: bildirim', e.message) }
  if (!isler.length) return
  bildirilen = gun
  const n = new Notification({
    title: 'Notix', icon: ICON,
    body: [`${isler.length} ${dil(cfg.lang)[3]}`, ...isler.slice(0, 3).map((x) => '• ' + x.text)].join('\n'),
  })
  n.on('click', () => { goster(); main?.webContents.executeJavaScript("document.getElementById('today')?.click()").catch(() => {}) })
  n.show()
}

// Pencereyi kapatmak uygulamayi kapatmasin: tepside kalir, global kisayol yasar.
// Cikis sadece tepsi menusunden.
let tray = null
let cikiyor = false
app.on('before-quit', () => { cikiyor = true })

// Windows bildirimleri uygulama kimligi olmadan gosterilmiyor (package.json build.appId ile ayni).
app.setAppUserModelId('com.mertokan.notix')

app.whenReady().then(() => {
  const win = main = new BrowserWindow({
    width: 1100, height: 780, minWidth: 420,
    title: 'Notix', backgroundColor: bg(true), icon: ICON,
  })
  win.loadURL(`http://127.0.0.1:${PORT}/?t=${TOKEN}`)
  win.webContents.setWindowOpenHandler(({ url }) => (shell.openExternal(url), { action: 'deny' }))
  win.on('close', (e) => { if (!cikiyor) { e.preventDefault(); win.hide() } })

  tray = new Tray(nativeImage.createFromPath(ICON).resize({ width: 16, height: 16 }))
  tray.setToolTip('Notix')
  const menu = (lang) => {
    const t = dil(lang)
    tray.setContextMenu(Menu.buildFromTemplate([
      { label: t[0], click: goster },
      { label: t[1], click: quickAdd },
      { type: 'separator' },
      { label: t[2], click: () => app.quit() },
    ]))
  }
  tray.on('click', goster)

  // Kisayol ayarlardan degisince yeniden baglanir; false donerse baska uygulama tutuyor.
  const bindQuick = (accel) => (globalShortcut.unregisterAll(), !!accel && globalShortcut.register(accel, quickAdd))
  // Acilista baslat: isletim sisteminin kendi kaydi (Windows'ta Run anahtari). Sadece kurulu surumde,
  // yoksa gelistirme sirasinda electron.exe'yi kayda yazar.
  const apply = (cfg) => {
    nativeTheme.themeSource = themeOf(cfg.theme)
    win.setBackgroundColor(bg(true))
    menu(cfg.lang)
    if (app.isPackaged) app.setLoginItemSettings({ openAtLogin: !!cfg.openAtLogin })
    return bindQuick(cfg.quickAdd)
  }
  if (!apply(config())) console.warn('notix: hizli ekleme kisayolu baska bir uygulamada')
  onConfig(apply)

  setTimeout(bildir, 10e3)
  setInterval(bildir, 36e5)
})

// window-all-closed yok: ana pencere kapanmiyor, gizleniyor. Cikis tepsiden.
