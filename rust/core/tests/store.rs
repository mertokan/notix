// test.js'in Rust karsiligi (sunucu/surum testleri haric: onlar server.js'e ait).
mod common;
use common::store;
use notix_core::{Code, Dir, Item, Kind, TaskUpdate};
use std::fs;

fn tasks(s: &notix_core::Store, slug: &str) -> Vec<(usize, bool, String)> {
    s.parse(slug)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|x| if let Item::Task { i, done, text, .. } = x { Some((i, done, text)) } else { None })
        .collect()
}

fn upd<'a>(expect: Option<&'a str>, done: Option<bool>, text: Option<&'a str>) -> TaskUpdate<'a> {
    TaskUpdate { expect, done, text, remove: false }
}

#[test]
fn yazilan_her_sey_korunur_sadece_hedef_satir_degisir() {
    let s = store();
    let slug = s.create("Servix Web").unwrap();
    assert_eq!(slug, "servix-web");
    let f = s.dir().join("servix-web.md");
    fs::write(&f, "# Servix Web\n\nAI notu: burasi elle yazilmis serbest metin, silinmemeli.\n- [ ] gateway timeout ayarla\n  - [x] alt gorev\n").unwrap();

    let p = s.parse(&slug).unwrap();
    assert_eq!(p.title, "Servix Web");
    assert_eq!(tasks(&s, &slug), [(3, false, "gateway timeout ayarla".into()), (4, true, "alt gorev".into())]);

    s.update_task(&slug, 3, upd(Some("gateway timeout ayarla"), Some(true), None)).unwrap();
    s.add_task(&slug, "yeni gorev", Kind::Task).unwrap();
    let after = fs::read_to_string(&f).unwrap();
    assert!(after.contains("AI notu: burasi elle yazilmis serbest metin, silinmemeli."));
    assert!(after.contains("- [x] gateway timeout ayarla"));
    assert!(after.lines().any(|l| l == "  - [x] alt gorev")); // girinti korunur
    assert!(after.contains("- [ ] yeni gorev"));
}

#[test]
fn arada_degisen_satiri_ezmez_409() {
    let s = store();
    let slug = s.create("Yaris").unwrap();
    s.add_task(&slug, "ilk", Kind::Task).unwrap();
    let i = tasks(&s, &slug)[0].0;
    s.update_task(&slug, i, upd(None, None, Some("AI bunu degistirdi"))).unwrap(); // baskasi yazdi
    let e = s.update_task(&slug, i, upd(Some("ilk"), Some(true), None)).unwrap_err();
    assert_eq!((e.code, e.status(), e.message.as_str()), (Code::Conflict, 409, "satir degismis"));
}

#[test]
fn hizli_ekleme_inbox_ikinci_kez_olusturulunca_sifirlanmaz() {
    let s = store();
    let slug = s.create("Inbox").unwrap();
    s.add_task(&slug, "ilk fikir", Kind::Task).unwrap();
    assert_eq!(s.create("Inbox").unwrap(), slug);
    s.add_task(&slug, "ikinci fikir", Kind::Task).unwrap();
    let t: Vec<_> = tasks(&s, &slug).into_iter().map(|x| x.2).collect();
    assert_eq!(t, ["ilk fikir", "ikinci fikir"]);
}

#[test]
fn turkce_harfler_slugda_kaybolmaz() {
    let s = store();
    assert_eq!(s.create("Ev İşleri").unwrap(), "ev-isleri");
    assert_eq!(notix_core::slugify("Çöp Güncesi ıI"), "cop-guncesi-ii");
}

#[test]
fn slug_disari_cikamaz() {
    let e = store().parse("../../etc/passwd").unwrap_err();
    assert_eq!((e.status(), e.message.as_str()), (400, "gecersiz proje adi"));
}

#[test]
fn ad_degistirme_sadece_baslik_satiri_ve_dosya_adi_degisir() {
    let s = store();
    let slug = s.create("Eski Ad").unwrap();
    s.add_task(&slug, "duran is", Kind::Task).unwrap();
    let eski = s.dir().join(format!("{slug}.md"));
    fs::write(&eski, fs::read_to_string(&eski).unwrap() + "elle yazilmis not\n").unwrap();

    let next = s.rename(&slug, "Yeni Ad").unwrap();
    assert_eq!(next, "yeni-ad");
    let after = fs::read_to_string(s.dir().join("yeni-ad.md")).unwrap();
    assert!(after.lines().any(|l| l == "# Yeni Ad"));
    assert!(after.contains("- [ ] duran is") && after.contains("elle yazilmis not"));
    assert!(!eski.exists());

    s.create("Cakisan").unwrap();
    let e = s.rename("yeni-ad", "Cakisan").unwrap_err();
    assert_eq!((e.status(), e.message.as_str()), (409, "bu isimde proje var"));
}

#[test]
fn alt_gorev_girintili_serbest_not_isaretsiz_eklenir() {
    let s = store();
    let slug = s.create("Satir Turleri").unwrap();
    s.add_task(&slug, "ana is", Kind::Task).unwrap();
    s.add_task(&slug, "alt is", Kind::Sub).unwrap();
    s.add_task(&slug, "sadece not", Kind::Note).unwrap();
    let items: Vec<(&str, String)> = s
        .parse(&slug)
        .unwrap()
        .items
        .iter()
        .filter(|x| !matches!(x, Item::Heading { .. }) && !x.text().trim().is_empty())
        .map(|x| (if matches!(x, Item::Task { .. }) { "task" } else { "text" }, x.text().to_string()))
        .collect();
    assert_eq!(items, [("task", "ana is".into()), ("task", "alt is".into()), ("text", "sadece not".into())]);
    assert!(s.read(&slug).unwrap().lines().any(|l| l == "  - [ ] alt is"));
}

#[test]
fn arama_tum_projelerde_buyuk_kucuk_harf_farketmez() {
    let s = store();
    s.import_file("arama-1", "# Arama 1\n- [ ] Kritik Kayit\n").unwrap();
    s.import_file("arama-2", "# Arama 2\n- [x] kritik olmayan\n").unwrap();
    let hits = s.search("KRITIK").unwrap();
    let mut slugs: Vec<_> = hits.iter().map(|h| h.slug.as_str()).collect();
    slugs.sort();
    assert_eq!(slugs, ["arama-1", "arama-2"]);
    assert_eq!(hits.iter().find(|h| h.slug == "arama-2").unwrap().done, Some(true));
    assert!(s.search("k").unwrap().is_empty()); // tek harf aranmaz
}

#[test]
fn ice_aktarma_eskisini_ezmeden_once_trash_e_kopyalar() {
    let s = store();
    s.import_file("tasinan", "# Tasinan\n- [ ] ilk surum\n").unwrap();
    let r = s.import_file("tasinan", "# Tasinan\n- [ ] ikinci surum\n").unwrap();
    assert!(r.replaced);
    assert!(s.read("tasinan").unwrap().contains("ikinci surum"));
    let yedek: Vec<_> = fs::read_dir(s.dir().join(".trash")).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with("tasinan-")).collect();
    assert_eq!(yedek.len(), 1);
    assert!(fs::read_to_string(yedek[0].path()).unwrap().contains("ilk surum"));
    assert_eq!(s.export_all().unwrap()["tasinan"], "# Tasinan\n- [ ] ikinci surum\n");
    // disaridan gelen ad temizlenir: klasor disina cikamaz, hicbir gecerli harf yoksa reddedilir
    assert_eq!(s.import_file("../kacak", "x").unwrap().slug, "kacak");
    assert_eq!(s.import_file("../..", "x").unwrap_err().status(), 400);
}

#[test]
fn bellek_bolum_yoksa_acilir_varsa_sadece_o_araliga_dokunur() {
    let s = store();
    let slug = s.create("Bellekli").unwrap();
    s.add_task(&slug, "duran is", Kind::Task).unwrap();
    let f = s.dir().join(format!("{slug}.md"));
    fs::write(&f, fs::read_to_string(&f).unwrap() + "## Notlar\n\nelle yazilmis\n").unwrap();

    s.set_memory(&slug, "Prod veritabani `x`.\nKarar: auth NextAuth.", None, None).unwrap();
    assert!(s.memory(&slug).unwrap().text.contains("Prod veritabani"));
    let dosya = fs::read_to_string(&f).unwrap();
    assert!(dosya.starts_with("# Bellekli\n")); // baslik yerinde
    assert!(dosya.contains("## Bellek") && dosya.contains("- [ ] duran is") && dosya.contains("## Notlar\n\nelle yazilmis"));

    // bellek en altta: yeni is onun ustune girer, bellege karismaz
    s.add_task(&slug, "sonradan eklenen is", Kind::Task).unwrap();
    assert!(!s.memory(&slug).unwrap().text.contains("sonradan eklenen is"));
    assert!(tasks(&s, &slug).iter().any(|t| t.2 == "sonradan eklenen is"));

    let m = s.memory(&slug).unwrap().text;
    s.set_memory(&slug, "Tek satir", Some(&m), None).unwrap(); // guncelleme
    assert_eq!(s.memory(&slug).unwrap().text, "Tek satir");
    assert!(fs::read_to_string(&f).unwrap().contains("## Notlar"));

    let e = s.set_memory(&slug, "baska", Some("eski hali"), None).unwrap_err();
    assert_eq!((e.status(), e.message.as_str()), (409, "bellek arada degismis"));

    s.set_memory(&slug, "", Some("Tek satir"), None).unwrap(); // bosaltinca bolum kalkar
    assert_eq!(s.memory(&slug).unwrap().from, -1);
    assert!(!fs::read_to_string(&f).unwrap().contains("## Bellek"));
}

#[test]
fn belge_modu_tum_dosya_yazilir_arada_degistiyse_409() {
    let s = store();
    let slug = s.create("Bellek").unwrap();
    s.add_task(&slug, "ilk is", Kind::Task).unwrap();
    let eski = s.read(&slug).unwrap();

    s.write(&slug, "# Bellek\n\nUzun not.\n\n![ekran](media/a-1.png)\n\n## Kararlar\n\n- [ ] ilk is\n", Some(&eski)).unwrap();
    let p = s.parse(&slug).unwrap();
    assert!(p.items.iter().any(|x| matches!(x, Item::Heading { level: 2, text, .. } if text == "Kararlar")));
    assert!(p.items.iter().any(|x| x.text() == "Uzun not."));
    assert_eq!(tasks(&s, &slug).len(), 1);

    assert_eq!(s.write(&slug, "# Bellek\n", Some(&eski)).unwrap_err().status(), 409);
}

#[test]
fn resim_sadece_bilinen_uzanti_klasor_disina_cikamaz() {
    let s = store();
    let yol = s.save_media("Ekran Görüntüsü.png", "ZGVuZW1l").unwrap(); // "deneme"
    let (stem, rest) = yol.strip_prefix("media/ekran-goruntusu-").unwrap().split_once('.').unwrap();
    assert!(stem.bytes().all(|b| b.is_ascii_digit()) && rest == "png");
    assert_eq!(fs::read(s.dir().join(&yol)).unwrap(), b"deneme");
    assert_eq!(s.media_file(&yol[6..]).unwrap().content_type, "image/png");
    assert_eq!(s.save_media("kotu.exe", "AA==").unwrap_err().status(), 400);
    assert_eq!(s.media_file("../../store.js").unwrap_err().status(), 400);
    assert_eq!(s.save_media("a.png", "").unwrap_err().message, "bos dosya");
    let buyuk = "A".repeat(7_000_000);
    let e = s.save_media("a.png", &buyuk).unwrap_err();
    assert_eq!((e.status(), e.message.as_str()), (413, "resim 5MB dan buyuk"));
}

#[test]
fn cop_kutusu_geri_alma_ad_cakismasi_ve_kalici_silme() {
    let s = store();
    let slug = s.create("Copluk").unwrap();
    s.add_task(&slug, "geri gelecek is", Kind::Task).unwrap();
    s.remove(&slug).unwrap();

    let bin: Vec<_> = s.trash().unwrap().into_iter().filter(|b| b.file.starts_with("copluk-")).collect();
    assert_eq!(bin.len(), 1);
    assert_eq!((bin[0].title.as_str(), bin[0].open), ("Copluk", 1));

    assert_eq!(s.restore(&bin[0].file).unwrap(), "copluk"); // eski adina doner
    assert!(s.read("copluk").unwrap().contains("geri gelecek is"));
    assert!(!s.trash().unwrap().iter().any(|b| b.file == bin[0].file));

    s.remove("copluk").unwrap();
    s.create("Copluk").unwrap(); // ad bosalinca yenisi acildi
    let yine = s.trash().unwrap().into_iter().find(|b| b.file.starts_with("copluk-")).unwrap();
    assert_eq!(s.restore(&yine.file).unwrap(), "copluk-2"); // ustune yazmaz

    s.remove("copluk-2").unwrap();
    let silinecek = s.trash().unwrap().into_iter().find(|b| b.file.starts_with("copluk-2-")).unwrap();
    s.purge(&silinecek.file).unwrap();
    assert!(!s.dir().join(".trash").join(&silinecek.file).exists());
    assert_eq!(s.purge("../../store.js").unwrap_err().status(), 400);
}

#[test]
fn proje_silme_trash_e_tasima() {
    let s = store();
    let slug = s.create("Gecici").unwrap();
    s.remove(&slug).unwrap();
    assert!(!s.list().unwrap().iter().any(|p| p.slug == slug));
    let n = fs::read_dir(s.dir().join(".trash")).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with("gecici-")).count();
    assert_eq!(n, 1);
}

#[test]
fn cok_satirli_yapistirma_oldugu_gibi_girer_girinti_korunur() {
    let s = store();
    let slug = s.create("Yapistirma").unwrap();
    s.add_task(&slug, "Sunucu kurulumu\n  - [ ] paketleri kur\n  - dns kaydi\n\nnot: staging once", Kind::Task).unwrap();
    let raw = s.read("yapistirma").unwrap();
    let lines: Vec<&str> = raw.split('\n').collect();
    let b = lines.iter().position(|l| *l == "- [ ] Sunucu kurulumu").unwrap();
    assert_eq!(lines[b..b + 5], ["- [ ] Sunucu kurulumu", "  - [ ] paketleri kur", "  - dns kaydi", "", "not: staging once"]);
    let items = s.parse(&slug).unwrap().items;
    assert!(items.iter().any(|x| matches!(x, Item::Task { text, indent: 2, .. } if text == "paketleri kur")));
    assert!(items.iter().any(|x| matches!(x, Item::Text { text, indent: 2, .. } if text == "- dns kaydi")));
}

#[test]
fn satir_tasima_komsu_gorevle_yer_degistirir_arasi_bozulmaz() {
    let s = store();
    let slug = s.create("Sira").unwrap();
    fs::write(s.dir().join("sira.md"), "# Sira\n- [ ] birinci\nserbest metin\n- [ ] ikinci\n").unwrap();
    s.move_task(&slug, 3, Dir::Up, Some("ikinci")).unwrap();
    assert_eq!(s.read("sira").unwrap().split('\n').collect::<Vec<_>>()[1..4], ["- [ ] ikinci", "serbest metin", "- [ ] birinci"]);
    s.move_task(&slug, 1, Dir::Up, Some("ikinci")).unwrap(); // liste basi: sessizce durur
    assert_eq!(s.parse(&slug).unwrap().items[1].text(), "ikinci");
    let e = s.move_task(&slug, 1, Dir::Down, Some("baska metin")).unwrap_err();
    assert_eq!(e.message, "satir degismis");
}

#[test]
fn baska_projeye_tasima_kaynaktan_gider_hedefe_girer() {
    let s = store();
    let (a, b) = (s.create("Kaynak").unwrap(), s.create("Hedef").unwrap());
    s.add_task(&a, "tasinacak is", Kind::Sub).unwrap();
    let i = tasks(&s, &a)[0].0;
    s.move_to_project(&a, i, &b, Some("tasinacak is")).unwrap();
    assert!(tasks(&s, &a).is_empty());
    let items = s.parse(&b).unwrap().items;
    assert!(items.iter().any(|x| matches!(x, Item::Task { text, indent: 0, .. } if text == "tasinacak is")));
    let e = s.move_to_project(&b, 1, "olmayan-proje", Some("tasinacak is")).unwrap_err();
    assert_eq!((e.status(), e.message.as_str()), (404, "hedef proje yok"));
    assert_eq!(s.move_to_project(&b, 1, &b, None).unwrap_err().message, "ayni proje");
}

#[test]
fn bugun_tarihi_gelmis_acik_isler_gelecektekiler_haric() {
    let s = store();
    let slug = s.create("Tarihli").unwrap();
    for t in ["gecmis is @2020-01-01", "bugunku is @2026-09-11", "ileri is @2099-01-01", "tarihsiz is"] {
        s.add_task(&slug, t, Kind::Task).unwrap();
    }
    let (i, _, text) = tasks(&s, &slug).into_iter().find(|x| x.2.starts_with("gecmis")).unwrap();
    let dates = |s: &notix_core::Store| s.due("2026-09-11").unwrap().into_iter().map(|x| x.date).collect::<Vec<_>>();
    assert_eq!(dates(&s), ["2020-01-01", "2026-09-11"]);
    s.update_task(&slug, i, upd(Some(&text), Some(true), None)).unwrap();
    assert_eq!(dates(&s), ["2026-09-11"]); // bitmisler dusuyor
}

#[test]
fn bellek_basligi_yoksa_verilen_dilde_acilir_varsa_dosyadaki_korunur() {
    let s = store();
    let slug = s.create("Baslik dili").unwrap();
    s.set_memory(&slug, "ilk", None, Some("Memory")).unwrap();
    assert!(s.read(&slug).unwrap().lines().any(|l| l == "## Memory"));
    s.set_memory(&slug, "ikinci", Some("ilk"), None).unwrap(); // varsayilan Bellek ama mevcut baslik degismez
    let r = s.read(&slug).unwrap();
    assert!(r.lines().any(|l| l == "## Memory") && !r.contains("Bellek"));
}

// --- store.js'te testi olmayan ama sozlesmenin parcasi olan davranislar ---
#[test]
fn bos_gorev_ve_bos_baslik_400() {
    let s = store();
    let slug = s.create("X").unwrap();
    assert_eq!(s.add_task(&slug, " \n\n ", Kind::Task).unwrap_err().message, "bos gorev");
    assert_eq!(s.create("???").unwrap_err().message, "baslik bos");
    assert_eq!(s.rename(&slug, "..").unwrap_err().message, "baslik bos");
}

#[test]
fn olmayan_dosya_missing_kodu_verir() {
    let e = store().parse("yok").unwrap_err();
    assert_eq!((e.code, e.status()), (Code::Missing, 404));
}

#[test]
fn crlf_okunur_lf_yazilir_ve_tanimadigi_satira_dokunulmaz() {
    let s = store();
    fs::write(s.dir().join("crlf.md"), "# Crlf\r\n\r\nserbest\r\n- [ ] a\r\n").unwrap();
    s.update_task("crlf", 3, upd(Some("a"), Some(true), None)).unwrap();
    assert_eq!(s.read("crlf").unwrap(), "# Crlf\n\nserbest\n- [x] a\n");
}

#[test]
fn memory_basligi_buyuk_kucuk_ve_sonraki_baslik_bolumu_bitirir() {
    let s = store();
    fs::write(s.dir().join("m.md"), "# M\n- [ ] a\n### MEMORY \nbir\n\n#### alt\niki\n## Baska\nuc\n").unwrap();
    assert_eq!(s.memory("m").unwrap().text, "bir\n\n#### alt\niki");
    s.set_memory("m", "yeni", None, None).unwrap();
    assert_eq!(s.read("m").unwrap(), "# M\n- [ ] a\n### MEMORY \n\nyeni\n\n## Baska\nuc\n");
}

#[test]
fn liste_acik_sayisina_gore_siralar_ve_gecersiz_adi_atlar() {
    let s = store();
    s.import_file("a", "# Alfa\n- [x] a\n").unwrap();
    s.import_file("b", "# Beta\n- [ ] b\n").unwrap();
    fs::write(s.dir().join("README.md"), "# not a project").unwrap();
    let l = s.list().unwrap();
    assert_eq!(l.iter().map(|x| x.slug.as_str()).collect::<Vec<_>>(), ["b", "a"]);
    assert_eq!((l[1].open, l[1].done), (0, 1));
}
