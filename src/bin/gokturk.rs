//! Modern Türkçeyi Old Turkic / Orhun harflerine yaklaşık dönüştüren küçük program.
//!
//! Önemli notlar:
//! - Bu program tarihî açıdan kusursuz Göktürkçe çeviri yapmaz.
//! - Old Turkic yazısı modern Türkçeye bire bir karşılık gelmez.
//! - Bazı consonantların kalın/ince ünlü uyumuna göre farklı biçimleri vardır.
//! - `f`, `h`, `j`, `v` gibi modern Türkçe harflerin doğrudan Old Turkic karşılığı yoktur;
//!   bu program onları olduğu gibi bırakır.
//! - Old Turkic Unicode bloğu: U+10C00..U+10C4F.
//! - Kelime ayırıcı olarak bazen U+205A TWO DOT PUNCTUATION `⁚` kullanılır.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VowelClass {
    Back,
    Front,
}

#[derive(Debug, Clone, Copy)]
struct TableRow {
    latin: &'static str,
    old_turkic: char,
    code: &'static str,
    unicode_name: &'static str,
    note: &'static str,
}

const TABLE: &[TableRow] = &[
    TableRow {
        latin: "a",
        old_turkic: '𐰀',
        code: "U+10C00",
        unicode_name: "OLD TURKIC LETTER ORKHON A",
        note: "kalın ünlü",
    },
    TableRow {
        latin: "e",
        old_turkic: '𐰅',
        code: "U+10C05",
        unicode_name: "OLD TURKIC LETTER YENISEI E",
        note: "ince ünlü; yaklaşık kullanım",
    },
    TableRow {
        latin: "ı/i",
        old_turkic: '𐰃',
        code: "U+10C03",
        unicode_name: "OLD TURKIC LETTER ORKHON I",
        note: "modern ı/i için yaklaşık kullanım",
    },
    TableRow {
        latin: "o/u",
        old_turkic: '𐰆',
        code: "U+10C06",
        unicode_name: "OLD TURKIC LETTER ORKHON O",
        note: "modern o/u için yaklaşık kullanım",
    },
    TableRow {
        latin: "ö/ü",
        old_turkic: '𐰇',
        code: "U+10C07",
        unicode_name: "OLD TURKIC LETTER ORKHON OE",
        note: "modern ö/ü için yaklaşık kullanım",
    },
    TableRow {
        latin: "b¹",
        old_turkic: '𐰉',
        code: "U+10C09",
        unicode_name: "OLD TURKIC LETTER ORKHON AB",
        note: "kalın çevre",
    },
    TableRow {
        latin: "b²",
        old_turkic: '𐰋',
        code: "U+10C0B",
        unicode_name: "OLD TURKIC LETTER ORKHON AEB",
        note: "ince çevre",
    },
    TableRow {
        latin: "g/ğ¹",
        old_turkic: '𐰍',
        code: "U+10C0D",
        unicode_name: "OLD TURKIC LETTER ORKHON AG",
        note: "kalın çevre",
    },
    TableRow {
        latin: "g/ğ²",
        old_turkic: '𐰏',
        code: "U+10C0F",
        unicode_name: "OLD TURKIC LETTER ORKHON AEG",
        note: "ince çevre",
    },
    TableRow {
        latin: "d¹",
        old_turkic: '𐰑',
        code: "U+10C11",
        unicode_name: "OLD TURKIC LETTER ORKHON AD",
        note: "kalın çevre",
    },
    TableRow {
        latin: "d²",
        old_turkic: '𐰓',
        code: "U+10C13",
        unicode_name: "OLD TURKIC LETTER ORKHON AED",
        note: "ince çevre",
    },
    TableRow {
        latin: "z",
        old_turkic: '𐰔',
        code: "U+10C14",
        unicode_name: "OLD TURKIC LETTER ORKHON EZ",
        note: "yaklaşık kullanım",
    },
    TableRow {
        latin: "y¹",
        old_turkic: '𐰖',
        code: "U+10C16",
        unicode_name: "OLD TURKIC LETTER ORKHON AY",
        note: "kalın çevre",
    },
    TableRow {
        latin: "y²",
        old_turkic: '𐰘',
        code: "U+10C18",
        unicode_name: "OLD TURKIC LETTER ORKHON AEY",
        note: "ince çevre",
    },
    TableRow {
        latin: "k/q¹",
        old_turkic: '𐰴',
        code: "U+10C34",
        unicode_name: "OLD TURKIC LETTER ORKHON AQ",
        note: "kalın k/q",
    },
    TableRow {
        latin: "k²",
        old_turkic: '𐰚',
        code: "U+10C1A",
        unicode_name: "OLD TURKIC LETTER ORKHON AEK",
        note: "ince k",
    },
    TableRow {
        latin: "l¹",
        old_turkic: '𐰞',
        code: "U+10C1E",
        unicode_name: "OLD TURKIC LETTER ORKHON AL",
        note: "kalın çevre",
    },
    TableRow {
        latin: "l²",
        old_turkic: '𐰠',
        code: "U+10C20",
        unicode_name: "OLD TURKIC LETTER ORKHON AEL",
        note: "ince çevre",
    },
    TableRow {
        latin: "m",
        old_turkic: '𐰢',
        code: "U+10C22",
        unicode_name: "OLD TURKIC LETTER ORKHON EM",
        note: "yaklaşık kullanım",
    },
    TableRow {
        latin: "n¹",
        old_turkic: '𐰣',
        code: "U+10C23",
        unicode_name: "OLD TURKIC LETTER ORKHON AN",
        note: "kalın çevre",
    },
    TableRow {
        latin: "n²",
        old_turkic: '𐰤',
        code: "U+10C24",
        unicode_name: "OLD TURKIC LETTER ORKHON AEN",
        note: "ince çevre",
    },
    TableRow {
        latin: "ng/ŋ",
        old_turkic: '𐰭',
        code: "U+10C2D",
        unicode_name: "OLD TURKIC LETTER ORKHON ENG",
        note: "velar nazal",
    },
    TableRow {
        latin: "p",
        old_turkic: '𐰯',
        code: "U+10C2F",
        unicode_name: "OLD TURKIC LETTER ORKHON EP",
        note: "yaklaşık kullanım",
    },
    TableRow {
        latin: "ç/c",
        old_turkic: '𐰲',
        code: "U+10C32",
        unicode_name: "OLD TURKIC LETTER ORKHON EC",
        note: "modern ç/c için yaklaşık kullanım",
    },
    TableRow {
        latin: "r¹",
        old_turkic: '𐰺',
        code: "U+10C3A",
        unicode_name: "OLD TURKIC LETTER ORKHON AR",
        note: "kalın çevre",
    },
    TableRow {
        latin: "r²",
        old_turkic: '𐰼',
        code: "U+10C3C",
        unicode_name: "OLD TURKIC LETTER ORKHON AER",
        note: "ince çevre",
    },
    TableRow {
        latin: "s¹",
        old_turkic: '𐰽',
        code: "U+10C3D",
        unicode_name: "OLD TURKIC LETTER ORKHON AS",
        note: "kalın çevre",
    },
    TableRow {
        latin: "s²",
        old_turkic: '𐰾',
        code: "U+10C3E",
        unicode_name: "OLD TURKIC LETTER ORKHON AES",
        note: "ince çevre",
    },
    TableRow {
        latin: "ş¹",
        old_turkic: '𐰿',
        code: "U+10C3F",
        unicode_name: "OLD TURKIC LETTER ORKHON ASH",
        note: "kalın çevre",
    },
    TableRow {
        latin: "ş²",
        old_turkic: '𐱁',
        code: "U+10C41",
        unicode_name: "OLD TURKIC LETTER ORKHON ESH",
        note: "ince çevre",
    },
    TableRow {
        latin: "t¹",
        old_turkic: '𐱃',
        code: "U+10C43",
        unicode_name: "OLD TURKIC LETTER ORKHON AT",
        note: "kalın çevre",
    },
    TableRow {
        latin: "t²",
        old_turkic: '𐱅',
        code: "U+10C45",
        unicode_name: "OLD TURKIC LETTER ORKHON AET",
        note: "ince çevre",
    },
    TableRow {
        latin: "ayırıcı",
        old_turkic: '⁚',
        code: "U+205A",
        unicode_name: "TWO DOT PUNCTUATION",
        note: "kelime ayırıcı olarak kullanılabilir; Old Turkic bloğunda değildir",
    },
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        return;
    }

    if args.iter().any(|arg| arg == "--tablo" || arg == "-t") {
        print_table();
        return;
    }

    let use_separator = args
        .iter()
        .any(|arg| arg == "--ayirici" || arg == "--separator");
    let input = args
        .iter()
        .filter(|arg| !matches!(arg.as_str(), "--ayirici" | "--separator"))
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");

    let output = to_old_turkic(&input, use_separator);

    println!("\n\x1b[0;32mGirdi:\x1b[0m\n{input}");
    println!("\n\x1b[0;34mOld Turkic / Orhun yaklaşık karşılığı:\x1b[0m");
    println!("\x1b[0;36m{output}\x1b[0m\n");
}

fn print_usage() {
    eprintln!("Kullanım:");
    eprintln!("  gokturk <metin>             # metni yaklaşık Orhun harflerine dönüştürür");
    eprintln!("  gokturk --ayirici <metin>   # boşlukları U+205A '⁚' ayırıcıya dönüştürür");
    eprintln!("  gokturk --tablo             # kullanılan karakter tablosunu gösterir");
    eprintln!();
    eprintln!("Not: Bu tarihî açıdan kusursuz Göktürkçe çeviri değildir.");
}

fn print_table() {
    println!("Old Turkic / Orhun Yaklaşık Karakter Tablosu\n");
    println!(
        "{:<10} {:<4} {:<10} {:<45} Not",
        "Latin", "Harf", "Kod", "Unicode adı"
    );
    println!("{}", "-".repeat(95));

    for row in TABLE {
        println!(
            "{:<10} {:<4} {:<10} {:<45} {}",
            row.latin, row.old_turkic, row.code, row.unicode_name, row.note
        );
    }

    println!("\nDesteklenmeyen modern harfler: f, h, j, v, x, w");
    println!(
        "Rakamlar: Old Turkic bloğunda 1-10 gibi ayrı rakam sembolleri yoktur; bu program rakamları aynen bırakır."
    );
    println!("Noktalama: Modern . , ! ? işaretleri aynen bırakılır.");
}

fn to_old_turkic(input: &str, use_separator: bool) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut converted = String::new();
    let mut i = 0;

    while i < chars.len() {
        let ch = lowercase_turkish(chars[i]);

        // Modern "ng" dizisini tek bir Old Turkic ENG harfine dönüştürmek için.
        if ch == 'n' && i + 1 < chars.len() && lowercase_turkish(chars[i + 1]) == 'g' {
            converted.push('𐰭');
            i += 2;
            continue;
        }

        if ch.is_whitespace() {
            converted.push(if use_separator { '⁚' } else { ' ' });
            i += 1;
            continue;
        }

        let class = nearest_vowel_class(&chars, i).unwrap_or(VowelClass::Back);
        let mapped = map_char(ch, class).unwrap_or(ch);
        converted.push(mapped);
        i += 1;
    }

    // Old Turkic sağdan sola yazılır. Birçok terminal bunu karışık gösterebildiği için
    // görsel amaçlı ters çeviriyoruz. İstersen bu satırı kaldırıp doğal Unicode bidi
    // davranışına bırakabilirsin.
    converted.chars().rev().collect()
}

fn map_char(ch: char, class: VowelClass) -> Option<char> {
    let mapped = match ch {
        'a' => '𐰀',
        'e' => '𐰅',
        'ı' | 'i' => '𐰃',
        'o' | 'u' => '𐰆',
        'ö' | 'ü' => '𐰇',

        'b' => choose(class, '𐰉', '𐰋'),
        'g' | 'ğ' => choose(class, '𐰍', '𐰏'),
        'd' => choose(class, '𐰑', '𐰓'),
        'y' => choose(class, '𐰖', '𐰘'),
        'k' | 'q' => choose(class, '𐰴', '𐰚'),
        'l' => choose(class, '𐰞', '𐰠'),
        'n' => choose(class, '𐰣', '𐰤'),
        'r' => choose(class, '𐰺', '𐰼'),
        's' => choose(class, '𐰽', '𐰾'),
        'ş' => choose(class, '𐰿', '𐱁'),
        't' => choose(class, '𐱃', '𐱅'),

        'm' => '𐰢',
        'p' => '𐰯',
        'z' => '𐰔',
        'ç' | 'c' => '𐰲',
        'ŋ' => '𐰭',

        _ => return None,
    };

    Some(mapped)
}

fn choose(class: VowelClass, back: char, front: char) -> char {
    match class {
        VowelClass::Back => back,
        VowelClass::Front => front,
    }
}

fn nearest_vowel_class(chars: &[char], index: usize) -> Option<VowelClass> {
    // Önce aynı kelimede sola doğru bakıyoruz; çoğu durumda consonanttan önceki ünlü belirleyicidir.
    for &ch in chars[..index].iter().rev() {
        if ch.is_whitespace() || is_punctuation(ch) {
            break;
        }
        if let Some(class) = vowel_class(lowercase_turkish(ch)) {
            return Some(class);
        }
    }

    // Bulunamazsa aynı kelimede sağa doğru bakıyoruz.
    for &ch in chars.iter().skip(index + 1) {
        if ch.is_whitespace() || is_punctuation(ch) {
            break;
        }
        if let Some(class) = vowel_class(lowercase_turkish(ch)) {
            return Some(class);
        }
    }

    None
}

fn vowel_class(ch: char) -> Option<VowelClass> {
    match ch {
        'a' | 'ı' | 'o' | 'u' => Some(VowelClass::Back),
        'e' | 'i' | 'ö' | 'ü' => Some(VowelClass::Front),
        _ => None,
    }
}

fn is_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '.' | ','
            | ';'
            | ':'
            | '!'
            | '?'
            | '\''
            | '"'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '-'
            | '—'
            | '–'
    )
}

fn lowercase_turkish(ch: char) -> char {
    match ch {
        'A' => 'a',
        'B' => 'b',
        'C' => 'c',
        'Ç' => 'ç',
        'D' => 'd',
        'E' => 'e',
        'F' => 'f',
        'G' => 'g',
        'Ğ' => 'ğ',
        'H' => 'h',
        'I' => 'ı',
        'İ' => 'i',
        'J' => 'j',
        'K' => 'k',
        'L' => 'l',
        'M' => 'm',
        'N' => 'n',
        'O' => 'o',
        'Ö' => 'ö',
        'P' => 'p',
        'R' => 'r',
        'S' => 's',
        'Ş' => 'ş',
        'T' => 't',
        'U' => 'u',
        'Ü' => 'ü',
        'V' => 'v',
        'Y' => 'y',
        'Z' => 'z',
        _ => ch,
    }
}
