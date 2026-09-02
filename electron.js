const { app, BrowserWindow, Menu, globalShortcut, nativeTheme, shell } = require('electron')
const path = require('node:path')

// Kurulu uygulamada kaynak klasoru salt okunur (app.asar): token/ayar userData'ya,
// notlar Belgeler/Notix'e. Gelistirirken (npm start) her sey repo icinde kalir.
if (app.isPackaged) {
  process.env.NOTIX_HOME ||= app.getPath('userData')
  process.env.NOTIX_DIR ||= path.join(app.getPath('documents'), 'Notix')
}
const { start, PORT, TOKEN, config, onConfig, onWindow } = require('./server')

// Acilir listeler/kaydirma cubuklari Chromium'un kendi ciziminde: sayfa CSS'i
// oraya gecmiyor, tema buradan zorlanmali.
nativeTheme.themeSource = 'dark'
// Varsayilan menu yok: Alt'a basinca gizli menu cubugu acilip (Dosya > Cikis)
// uygulama kapanabiliyordu. Kopyala/yapistir Chromium'da menusuz de calisiyor.
Menu.setApplicationMenu(null)

// Ikinci kez acilirsa port zaten dolu olurdu (pencere "baglanamadi" ile acilir);
// yeni ornek cikar, var olanin penceresini one getirir.
let main = null
if (!app.requestSingleInstanceLock()) { app.quit(); return }
app.on('second-instance', () => {
  if (!main) return
  if (main.isMinimized()) main.restore()
  main.focus()
})

start()

// ponytail: kutu her kisayolda sifirdan acilir (~150ms). Gec geldigi hissedilirse
// gizli pencere tutup show()/hide() yap.
let quick = null
function quickAdd() {
  if (quick) return quick.focus()
  quick = new BrowserWindow({
    width: 560, height: 62, frame: false, resizable: false, alwaysOnTop: true,
    skipTaskbar: true, backgroundColor: '#161922', show: false,
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

app.whenReady().then(() => {
  const win = main = new BrowserWindow({
    width: 1100, height: 780, minWidth: 420,
    title: 'Notix', backgroundColor: '#191713',
  })
  win.loadURL(`http://127.0.0.1:${PORT}/?t=${TOKEN}`)
  win.webContents.setWindowOpenHandler(({ url }) => (shell.openExternal(url), { action: 'deny' }))

  // Kisayol ayarlardan degisince yeniden baglanir; false donerse baska uygulama tutuyor.
  const bindQuick = (accel) => (globalShortcut.unregisterAll(), !!accel && globalShortcut.register(accel, quickAdd))
  // Acilista baslat: isletim sisteminin kendi kaydi (Windows'ta Run anahtari). Sadece kurulu surumde,
  // yoksa gelistirme sirasinda electron.exe'yi kayda yazar.
  const apply = (cfg) => {
    if (app.isPackaged) app.setLoginItemSettings({ openAtLogin: !!cfg.openAtLogin })
    return bindQuick(cfg.quickAdd)
  }
  if (!apply(config())) console.warn('notix: hizli ekleme kisayolu baska bir uygulamada')
  onConfig(apply)
})

app.on('window-all-closed', () => app.quit())
