# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## İki farklı iş

Bu repo hem bir uygulama hem bir veri deposu. Hangi işi yaptığını ayırt et:

1. **Notları kullanmak/güncellemek** (asıl amaç) — `notes/*.md` dosyalarını doğrudan
   oku/düzenle. Kurallar: @AGENTS.md. Kodu çalıştırmana gerek yok.
2. **Notix'i geliştirmek** — aşağısı.

`notes/` kullanıcının verisidir. Formatını değiştirme, yeniden biçimlendirme,
"düzeltme". `notes/.trash/`, `.notix-token` ve `.notix-config.json` git dışıdır.

## Komutlar

```bash
npm start                                  # masaüstü uygulaması (Electron; server'ı kendi başlatır)
npm run server                             # sadece HTTP sunucu, 127.0.0.1:4321
npm test                                   # tüm testler (node:test, framework yok)
npm run dist                               # Windows kurulum dosyası (electron-builder, NSIS) → dist/
node --test --test-name-pattern="409"      # tek test
```

Electron 43'te binary indirme postinstall'dan ayrıldı: `node_modules/electron/dist`
boşsa `npx install-electron --no` çalıştır.

**Claude Code içinden Electron başlatırken:** ortamda `ELECTRON_RUN_AS_NODE=1`
miras kalıyor, bu haldeyken `require('electron')` API yerine yol string'i döner ve
`app.whenReady` undefined patlar. `env -u ELECTRON_RUN_AS_NODE ./node_modules/electron/dist/electron.exe .`
ile çalıştır.

## Mimari

Kaynak veri **markdown dosyalarıdır**, veritabanı veya JSON değil. Sebebi: AI
ajanları hiçbir entegrasyon olmadan listeyi okuyup işleri işaretleyebilsin.
Bu karar her şeyi belirliyor — bir "gerçek" veri modeli/ORM/şema eklemek amacı bozar.

```
notes/<slug>.md  →  store.js  →  server.js (127.0.0.1)  →  public/ (vanilla JS)
                                      ↑
                                 electron.js (pencere + sunucu aynı process)
```

- **store.js** — tek gerçek mantık. Dosyayı **satır dizisi** olarak tutar; sadece
  hedef satırı değiştirir, tanımadığı satırlara (başlık, serbest metin, girinti,
  boş satır) dokunmaz. Dosyayı komple yeniden üreten bir serializer yazma — elle
  veya AI tarafından yazılmış içerik kaybolur.
- **Optimistic concurrency**: mutasyonlar `expect` alır (istemcinin gördüğü satır
  metni). Uymuyorsa `409` — dosya arada değişmiş demektir (AI veya editör yazmış).
  Bu, çakışan yazımların birbirini ezmesini engelleyen tek mekanizma; kaldırma.
- **Silme yok**: proje silme `notes/.trash/`'e taşır, görev satırı silinir ama
  bitmiş işler `[x]` olarak kalır. Tek gerçek silme `store.purge()` — sadece çöp
  kutusundan, kullanıcı onaylayınca. Geri alma üzerine yazmaz, adı doluysa `slug-2`.
- **server.js** — `node:http`, elle routing, sıfır bağımlılık. `/api/*` token ister,
  gerisi `public/` statik. Sadece `127.0.0.1` dinler; token buna rağmen duruyor
  çünkü localhost portuna tarayıcıdaki herhangi bir sayfa da POST atabilir.
  Kısayollar `.notix-config.json`'da; `PUT /api/config` sadece bilinen alanları
  yazar ve `onConfig` ile electron.js'e haber verir — global kısayol yeniden
  başlatmadan yeniden bağlanır.
- **Veri taşıma**: `GET /api/export` tüm dosyaları JSON olarak verir, `POST /api/import`
  geri yazar. İçe aktarma üzerine yazmadan önce eskisini `.trash`'e kopyalar —
  "silme yok" kuralı burada da geçerli.
- **Proje belleği**: `## Bellek` bölümü (`GET/PUT /api/projects/:slug/memory`).
  Ayrı dosya değil — ajanlar zaten proje dosyasını okuyor. Bölüm dosyanın
  **sonunda** durur: "sonraki başlığa kadar" kuralı yüzünden ortada olsa altındaki
  başlıksız her şeyi yutardı. `addTask` bu yüzden yeni satırı bölümün üstüne yazar.
- **Belge modu**: `GET/PUT /api/projects/:slug/raw` dosyanın tamamını okur/yazar
  (`expect` = açılıştaki içerik, uymazsa 409 ve editör kapanmaz). Satır satır
  eklenemeyen her şey — çok satırlı not, bölüm başlığı, resim, projenin belleği —
  buradan yazılır. Resimler `notes/media/`, `GET /media/<dosya>?t=` ile servis edilir
  (`<img>` başlık gönderemiyor); sadece bilinen uzantılar, 5MB sınırı.
- **public/md.js** — markdown'ın satır içinde gösterimi + yazma kısayolları; iki
  sayfa da (ana pencere, hızlı ekleme) aynı dosyayı `<script>` ile alır. Bağ/resim
  adresleri `http(s)` veya `media/…` değilse düz metin kalır (`javascript:` geçmez).
- **electron.js** — tek örnek kilidi (ikinci açılış porta çakışmasın), menü yok
  (Alt gizli menüyü açıp "Çıkış"ı tetikleyebiliyordu), `nativeTheme.themeSource='dark'`.
  Hızlı ekleme kutusundaki proje listesi sayfanın içinde: sistemin açılır menüsü
  pencerenin dışına çiziliyor ve CSS almıyordu. Liste açılınca pencere büyüsün diye
  `POST /api/window` → `onWindow` — ayrı bir IPC/preload yerine zaten aynı process'te
  olan sunucu kullanılıyor.
- **Kurulu sürüm**: `app.isPackaged` ise electron.js `NOTIX_HOME`=userData (token, ayar)
  ve `NOTIX_DIR`=Belgeler/Notix verir; app.asar salt okunur. Ayarlardaki
  `openAtLogin` → `app.setLoginItemSettings` (sadece paketliyken, yoksa electron.exe
  kayda girer). `autoUpdate` → açılışta `GET /api/update` GitHub releases'tan son
  sürümü okur, sadece "yeni sürüm var" der; indirme/kurma yok (electron-updater sonra).
- **public/** — vanilla JS + DOM API, **build adımı yok**. Framework/bundler ekleme.
  Kullanıcı metni her zaman `textContent` ile basılır (`el()` helper), `innerHTML`
  ile değil — içeriği AI de yazıyor. 5 saniyede bir poll ile tazelenir (input
  odaktayken atlanır), böylece dosyayı dışarıdan değiştiren AI ekranda görünür.

## Bağımlılıklar

Runtime bağımlılığı **sıfır** (hepsi node stdlib), tek devDependency `electron`.
Bu kasıtlı. Paket eklemek bir karar — birkaç satırla çözülen şey için ekleme.
Eklenecekse en güncel sürüm (`npm view <pkg> version`).

## Kapsam dışı bırakılanlar

- **Mobil / PWA / LAN erişimi**: hesap-bulut olmadan ev dışından erişilemediği için
  kaldırıldı. Geri gelirse sunucu bind'ı `0.0.0.0` + manifest, ya da gerçek senkron.
- **MCP sunucusu**: dosya erişimi olan ajanlar zaten `notes/` okuyabiliyor.
