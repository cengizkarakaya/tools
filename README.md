# bookgrep

`bookgrep`, PDF ve EPUB kitap/doküman arşivlerinde kelime veya cümle aramak için yazılmış Rust tabanlı bir CLI aracıdır. Sonuçlarda dosya adı, sayfa veya bölüm bilgisi, kısa bağlam ve mümkün olduğunda kitap metadata bilgileri gösterilir.

## Kurulum

```powershell
cargo build --bin bookgrep
```

pCloud API desteği için:

```powershell
cargo build --bin bookgrep --features pcloud
```

## Örnek Kullanım

Lokal klasörde recursive arama:

```powershell
cargo run --bin bookgrep -- search --path ./books --query "ownership" --recursive
```

JSON çıktı:

```powershell
cargo run --bin bookgrep -- search --path ./books --query "ownership model" --recursive --json
```

Metadata ile çıktı:

```powershell
cargo run --bin bookgrep -- search --path ./books --query "borrowing" --recursive --metadata
```

Sadece EPUB arama:

```powershell
cargo run --bin bookgrep -- search --path ./books --query "memory" --recursive --extension epub
```

PDF veya EPUB bilgisi:

```powershell
cargo run --bin bookgrep -- info --file ./books/Rust.epub
```

## Lokal ve pCloud Drive

pCloud web arayüzündeki adresler doğrudan dosya URL'si değildir:

```text
https://my.pcloud.com//#/filemanager?folder=7825419682
```

pCloud Drive veya pCloud Sync ile klasör sisteme bağlandıysa `bookgrep` bunu normal lokal klasör gibi tarar:

```powershell
cargo run --bin bookgrep -- search --path "P:\Kitaplar" --query "ownership model" --recursive
```

Linux örneği:

```bash
cargo run --bin bookgrep -- search --path "/home/cengiz/pCloudDrive/Kitaplar" --query "ownership model" --recursive
```

## pCloud API

pCloud API desteği opsiyonel `pcloud` feature'ı arkasındadır. Token koda gömülmez; environment variable veya config dosyasından okunur.

PowerShell:

```powershell
$env:BOOKGREP_PCLOUD_TOKEN = "pc_xxx"
cargo run --bin bookgrep --features pcloud -- search --pcloud-folder-id 7825419682 --query "ownership model" --recursive
```

Remote path ile:

```powershell
cargo run --bin bookgrep --features pcloud -- search --pcloud-path "/Kitaplar" --query "ownership model"
```

Config dosyası platformun standart config klasöründe `bookgrep/config.json` olarak okunur:

```json
{
  "pcloud_token": "pc_xxx",
  "cache_dir": "C:/tmp/bookgrep-cache"
}
```

pCloud API modu `listfolder` ile klasör içeriğini listeler, sadece `.pdf` ve `.epub` dosyalarını arama hedefi yapar, indirilen dosyaları cache klasöründe saklar ve dosya boyutu/cache anahtarı ile tekrar indirmeyi azaltır.

## OPF Metadata

PDF dosyasının bulunduğu klasörde aynı ada sahip `.opf` varsa okunur:

```text
Kitap.pdf
Kitap.opf
```

Aynı klasörde tek bir `.opf` dosyası varsa o da metadata adayı kabul edilir. Başlık, yazar, yayıncı, tarih, dil, identifier, subject/tag, açıklama ve Calibre series bilgisi okunmaya çalışılır.

## Bilinen Sınırlamalar

OCR desteklenmez. Taranmış görsel PDF dosyalarında metin bulunamayabilir.

PDF text extraction Rust ekosisteminde her PDF için kusursuz değildir. Şifreli, bozuk veya görsel ağırlıklı PDF dosyalarında anlamlı hata döndürülür.

İlk sürüm doğrudan tarama yapar. `index` ve `search-index` komutları ileride Tantivy benzeri bir full-text index katmanı için ayrılmıştır.

Çok büyük PDF/EPUB dosyalarında extraction crate'leri bazı içerikleri belleğe alabilir. Kod modüler tutuldu; ileride streaming extraction ve OCR feature'ları eklenebilir.

## Geliştirme Komutları

```powershell
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --bin bookgrep -- search --path ./books --query "ownership"
```
