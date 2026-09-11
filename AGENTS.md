# Notix — AI ajanları için kullanım

Notix'in veri kaynağı **API değil, düz markdown dosyalarıdır**: `notes/<proje-slug>.md`.
Dosyaları doğrudan oku ve düzenle. Sunucu/uygulama bu dosyaları anlık yansıtır (5sn'de bir tazeler).

## Format

```markdown
# Proje Adı

Serbest metin satırları not olarak görünür. **Kalın**, `kod`, [bağlantı](https://…)
ve ![resim](media/dosya.png) uygulamada da öyle görünür.

- [ ] yapılacak iş
- [x] bitmiş iş
  - [ ] girintili alt iş (girinti korunur)
- [ ] tarihli iş @2026-09-15

## Bellek

Projenin hafızası. Uygulamada en üstte ayrı bir panelde görünür.
```

Kurallar:
- `- [ ]` → açık, `- [x]` → bitmiş. Başka bir işaret kullanma.
- Tarih = satırın içinde `@YYYY-MM-DD`. Tarihi bugün veya geçmiş olan açık işler
  uygulamanın "Bugün" listesinde toplanır. Başka bir tarih biçimi kullanma.
- **Satır bazlı düzenle.** Tanımadığın satırları (başlık, boş satır, serbest metin) aynen bırak.
- Bir işi bitirdiğinde satırı silme, sadece `[ ]` → `[x]` yap. Geçmiş kaybolmasın.
- Yeni iş = dosyanın sonuna yeni bir `- [ ]` satırı. Dosyanın sonunda `## Bellek`
  bölümü varsa işi **onun üstüne** ekle.
- Yeni proje = `notes/<slug>.md`, ilk satır `# Proje Adı`. Slug: küçük harf, rakam ve `-`.
- Dosya silme. Uygulama silmeleri `notes/.trash/` içine taşır; sen de öyle yap.

## Proje belleği

`## Bellek` başlığı altındaki bölüm projenin hafızasıdır: kararlar, bağlam,
bağlantılar, açık sorular. Uygulamada proje ekranının en üstünde ayrı bir panelde
görünür ve dosyanın **sonunda** durur (bölüm, sonraki aynı/üst seviye başlığa —
yoksa dosya sonuna — kadar sürer).

Bir projede çalışırken önce burayı oku. Öğrendiğin kalıcı bilgiyi (neden şöyle
yapıldı, hangi yol denenip bırakıldı, nerede duruyor) buraya yaz — görev listesine
değil. Var olan satırları silme, ekle veya güncelle.

## Tipik akış

1. `notes/` altındaki dosyaları oku, `## Bellek` bölümüyle bağlamı al, açık
   (`- [ ]`) işleri gör.
2. İşi yap.
3. İlgili satırı `- [x]` yap; gerekiyorsa altına açıklama satırı ekle. Kalıcı bir
   şey öğrendiysen `## Bellek`'e yaz.

## HTTP API (dosya erişimi olmayan ajanlar için)

Sunucu çalışıyorsa (`npm run server`, sadece `127.0.0.1:4321`), her istekte
`x-notix-token: <.notix-token dosyasının içeriği>` başlığı gerekir.

| | |
|---|---|
| `GET /api/projects` | proje listesi + açık iş sayısı |
| `GET /api/projects/:slug` | satır satır içerik (`i` = satır no) |
| `POST /api/projects` | `{title}` |
| `PATCH /api/projects/:slug` | `{title}` — başlık satırı + dosya adı değişir |
| `POST /api/projects/:slug/tasks` | `{text, kind?}` — `kind`: `task` (varsayılan), `sub` (girintili), `note` (serbest metin) |
| `PATCH /api/projects/:slug/tasks/:i` | `{expect, done?, text?}` — `{expect, move:"up"\|"down"}` sırala, `{expect, to:"hedef-slug"}` başka projeye taşı |
| `GET /api/search?q=` | tüm projelerde satır ara (en az 2 harf) |
| `GET /api/due?d=YYYY-MM-DD` | tarihi o güne kadar gelmiş açık işler (tüm projeler) |
| `DELETE /api/projects/:slug/tasks/:i?expect=…` | |
| `GET /api/export` | `{version, files: {slug: içerik}}` |
| `POST /api/import` | `{files: {slug: içerik}}` — var olanın eskisi `.trash`'e kopyalanır |
| `GET/PUT /api/config` | kısayollar |
| `GET /api/projects/:slug/memory` | `## Bellek` bölümü (`{text, from, to}`) |
| `PUT /api/projects/:slug/memory` | `{text, expect}` — bölümü yaz (yoksa açar, boş metin siler) |
| `GET /api/projects/:slug/raw` | dosyanın tamamı (`{text}`) |
| `PUT /api/projects/:slug/raw` | `{text, expect}` — tüm dosyayı yaz; `expect` okuduğun içerik, uymazsa 409 |
| `POST /api/media` | `{name, data}` (base64 resim) → `{path: "media/…"}` |
| `GET /api/trash` | çöp kutusundaki dosyalar |
| `POST /api/trash/:dosya/restore` | geri al (ad çakışırsa `slug-2` olur, üzerine yazmaz) |
| `DELETE /api/trash/:dosya` | kalıcı sil — uygulamadaki tek gerçek silme, geri alınamaz |

`expect` = değiştirmek istediğin satırın şu anki metni. Uymazsa `409` döner —
dosya arada değişmiş demektir, yeniden oku.
