# Notix

Proje notları / yapılacaklar. Masaüstü uygulaması, tamamen local, hesap yok.
Kaynak veri **`notes/<proje>.md`** — düz markdown, `- [ ]` / `- [x]`.
Dolayısıyla AI ajanları (Claude Code vb.) hiçbir entegrasyon olmadan listeyi
okuyup işleri işaretleyebilir → [AGENTS.md](AGENTS.md).

## Çalıştırma

Masaüstündeki **Notix** kısayolu. Ya da:

```bash
npm install          # tek bağımlılık: electron
npm start            # masaüstü uygulaması
npm run server       # sadece sunucu, tarayıcıdan kullanmak için (127.0.0.1:4321)
npm test             # store testleri
npm run dist         # Windows kurulum dosyası → dist/Notix Setup x.y.z.exe
```

## Ortam değişkenleri

| | |
|---|---|
| `NOTIX_DIR` | not klasörü (varsayılan `./notes`; kurulu uygulamada `Belgeler/Notix`) |
| `NOTIX_HOME` | token ve ayar dosyasının yeri (varsayılan repo kökü; kurulu uygulamada userData) |
| `NOTIX_UPDATE_URL` | güncelleme denetimi için son sürüm JSON'u (varsayılan GitHub releases API) |
| `NOTIX_PORT` | varsayılan `4321` |
| `NOTIX_TOKEN` | varsayılan: `.notix-token` içinde otomatik üretilir |

## Yedek

`notes/` klasörünü git'e commit'le — geçmiş + yedek bedava. Silinen projeler
`notes/.trash/` altına taşınır, gerçekten silinmez.

## Sonraya bırakılanlar

- **Mobil**: hesap/bulut olmadan ev dışından erişilemediği için ertelendi.
  Eklenince: sunucuyu `0.0.0.0`'a bağla + PWA manifest'i geri koy (LAN içi),
  ya da gerçek senkron istiyorsan bir hesap katmanı.
