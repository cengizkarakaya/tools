// Basit kullanım odaklı birim + döviz dönüştürücü:
// örnek: uco 5 km m
// örnek: uco 100 usd try
// Güvenli yaklaşım: tek sorumluluklu fonksiyonlar, doğrulama, anlamlı hata mesajları.

use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::time::Duration;

const ABSOLUTE_ZERO_C: f64 = -273.15;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const FRANKFURTER_LATEST_URL: &str = "https://api.frankfurter.app/latest";
const KIB: f64 = 1024.0;
const MIB: f64 = 1024.0 * 1024.0;
const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
const TIB: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0;

const HELP_TEXT: &str = r#"uco - Birim ve döviz dönüştürücü

Kullanım:
  uco <değer> <kaynak_birim> <hedef_birim>
  uco --help

Örnekler:
  uco 5 km m
  uco 100 usd try
  uco 2.5 bar psi
  uco 90 deg rad
  uco 1 kwh j

Desteklenen yerel birimler:
  Uzunluk:      mm, cm, m, km, in, ft, yd, mi
  Kütle:        mg, g, kg, ton, oz, lb
  Sıcaklık:     c, f, k
  Veri:         b, kb, mb, gb, tb, kib, mib, gib, tib
  Alan:         mm2, cm2, m2, km2, ha, acre, ft2, in2
  Hacim:        ml, l, m3, cm3, ft3, gal
  Zaman:        ns, us, ms, s, min, h, day, week
  Hız:          mps (m/s), kph (km/h), mph, knot
  Basınç:       pa, kpa, bar, atm, psi, torr
  Enerji:       j, kj, cal, kcal, wh, kwh, ev
  Güç:          w, kw, hp
  Kuvvet:       n, knf, dyn, kgf, lbf
  Açı:          rad, deg, grad, arcmin, arcsec
  Frekans:      hz, khz, mhz, ghz

Döviz:
  3 harfli ISO kodları için Frankfurter API kullanılır (örn. usd -> try).
"#;

fn print_help() {
    println!("{HELP_TEXT}");
}

struct Cli {
    /// Dönüştürülecek sayısal değer (örn: 5)
    value: f64,
    /// Kaynak birim/kod (örn: km, usd)
    from: String,
    /// Hedef birim/kod (örn: m, try)
    to: String,
}

impl Cli {
    fn parse() -> Self {
        match Self::try_parse_from(env::args()) {
            Ok(cli) => cli,
            Err(e) => {
                eprintln!("Hata: {e}");
                eprintln!("Kullanım: uco <değer> <kaynak_birim> <hedef_birim>");
                std::process::exit(2);
            }
        }
    }

    fn try_parse_from<I, T>(args: I) -> Result<Self, AppError>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut it = args.into_iter().map(Into::into);
        let _program = it.next();

        let value_raw = it
            .next()
            .ok_or(AppError::InvalidValue("değer argümanı eksik"))?;

        if matches!(value_raw.as_str(), "-h" | "--help" | "help") {
            print_help();
            std::process::exit(0);
        }

        let from = it
            .next()
            .ok_or(AppError::InvalidValue("kaynak birim argümanı eksik"))?;
        let to = it
            .next()
            .ok_or(AppError::InvalidValue("hedef birim argümanı eksik"))?;

        if it.next().is_some() {
            return Err(AppError::InvalidValue("fazla argüman verildi"));
        }

        let value = value_raw
            .parse::<f64>()
            .map_err(|_| AppError::InvalidValue("değer sayıya çevrilemedi"))?;

        Ok(Self { value, from, to })
    }
}

#[derive(Debug)]
enum AppError {
    InvalidValue(&'static str),
    UnknownUnit(String),
    IncompatibleUnits(String, String),
    Network(String),
    Api(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidValue(m) => write!(f, "geçersiz değer: {m}"),
            Self::UnknownUnit(u) => write!(f, "tanınmayan birim: {u}"),
            Self::IncompatibleUnits(a, b) => {
                write!(f, "uyumsuz birimler: {a} -> {b} (aynı kategoride olmalı)")
            }
            Self::Network(m) => write!(f, "ağ hatası: {m}"),
            Self::Api(m) => write!(f, "API hatası: {m}"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum UnitKind {
    Length,
    Weight,
    Temperature,
    Data,
    Area,
    Volume,
    Time,
    Speed,
    Pressure,
    Energy,
    Power,
    Force,
    Angle,
    Frequency,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Unit {
    // Length
    Mm,
    Cm,
    M,
    Km,
    Inch,
    Ft,
    Yard,
    Mile,
    // Weight
    Mg,
    G,
    Kg,
    Ton,
    Oz,
    Lb,
    // Temperature
    C,
    F,
    K,
    // Data
    B,
    KB,
    MB,
    GB,
    TB,
    KiB,
    MiB,
    GiB,
    TiB,
    // Area
    M2,
    Km2,
    Cm2,
    Mm2,
    Ha,
    Acre,
    Ft2,
    In2,
    // Volume
    Ml,
    L,
    M3,
    Cm3,
    Ft3,
    Gal,
    // Time
    Ns,
    Us,
    Ms,
    S,
    Min,
    H,
    Day,
    Week,
    // Speed
    Mps,
    Kph,
    Mph,
    Knot,
    // Pressure
    Pa,
    KPa,
    Bar,
    Atm,
    Psi,
    Torr,
    // Energy
    J,
    KJ,
    Cal,
    Kcal,
    Wh,
    KWh,
    Ev,
    // Power
    W,
    KW,
    Hp,
    // Force
    N,
    KN,
    Dyn,
    Kgf,
    Lbf,
    // Angle
    Rad,
    Deg,
    Grad,
    ArcMin,
    ArcSec,
    // Frequency
    Hz,
    KHz,
    MHz,
    GHz,
}

impl Unit {
    fn kind(self) -> UnitKind {
        match self {
            Unit::Mm
            | Unit::Cm
            | Unit::M
            | Unit::Km
            | Unit::Inch
            | Unit::Ft
            | Unit::Yard
            | Unit::Mile => UnitKind::Length,
            Unit::Mg | Unit::G | Unit::Kg | Unit::Ton | Unit::Oz | Unit::Lb => UnitKind::Weight,
            Unit::C | Unit::F | Unit::K => UnitKind::Temperature,
            Unit::B
            | Unit::KB
            | Unit::MB
            | Unit::GB
            | Unit::TB
            | Unit::KiB
            | Unit::MiB
            | Unit::GiB
            | Unit::TiB => UnitKind::Data,
            Unit::M2
            | Unit::Km2
            | Unit::Cm2
            | Unit::Mm2
            | Unit::Ha
            | Unit::Acre
            | Unit::Ft2
            | Unit::In2 => UnitKind::Area,
            Unit::Ml | Unit::L | Unit::M3 | Unit::Cm3 | Unit::Ft3 | Unit::Gal => UnitKind::Volume,
            Unit::Ns
            | Unit::Us
            | Unit::Ms
            | Unit::S
            | Unit::Min
            | Unit::H
            | Unit::Day
            | Unit::Week => UnitKind::Time,
            Unit::Mps | Unit::Kph | Unit::Mph | Unit::Knot => UnitKind::Speed,
            Unit::Pa | Unit::KPa | Unit::Bar | Unit::Atm | Unit::Psi | Unit::Torr => {
                UnitKind::Pressure
            }
            Unit::J | Unit::KJ | Unit::Cal | Unit::Kcal | Unit::Wh | Unit::KWh | Unit::Ev => {
                UnitKind::Energy
            }
            Unit::W | Unit::KW | Unit::Hp => UnitKind::Power,
            Unit::N | Unit::KN | Unit::Dyn | Unit::Kgf | Unit::Lbf => UnitKind::Force,
            Unit::Rad | Unit::Deg | Unit::Grad | Unit::ArcMin | Unit::ArcSec => UnitKind::Angle,
            Unit::Hz | Unit::KHz | Unit::MHz | Unit::GHz => UnitKind::Frequency,
        }
    }

    fn short(self) -> &'static str {
        match self {
            Unit::Mm => "mm",
            Unit::Cm => "cm",
            Unit::M => "m",
            Unit::Km => "km",
            Unit::Inch => "in",
            Unit::Ft => "ft",
            Unit::Yard => "yd",
            Unit::Mile => "mi",
            Unit::Mg => "mg",
            Unit::G => "g",
            Unit::Kg => "kg",
            Unit::Ton => "ton",
            Unit::Oz => "oz",
            Unit::Lb => "lb",
            Unit::C => "c",
            Unit::F => "f",
            Unit::K => "k",
            Unit::B => "b",
            Unit::KB => "kb",
            Unit::MB => "mb",
            Unit::GB => "gb",
            Unit::TB => "tb",
            Unit::KiB => "kib",
            Unit::MiB => "mib",
            Unit::GiB => "gib",
            Unit::TiB => "tib",
            Unit::M2 => "m2",
            Unit::Km2 => "km2",
            Unit::Cm2 => "cm2",
            Unit::Mm2 => "mm2",
            Unit::Ha => "ha",
            Unit::Acre => "acre",
            Unit::Ft2 => "ft2",
            Unit::In2 => "in2",
            Unit::Ml => "ml",
            Unit::L => "l",
            Unit::M3 => "m3",
            Unit::Cm3 => "cm3",
            Unit::Ft3 => "ft3",
            Unit::Gal => "gal",
            Unit::Ns => "ns",
            Unit::Us => "us",
            Unit::Ms => "ms",
            Unit::S => "s",
            Unit::Min => "min",
            Unit::H => "h",
            Unit::Day => "day",
            Unit::Week => "week",
            Unit::Mps => "m/s",
            Unit::Kph => "km/h",
            Unit::Mph => "mph",
            Unit::Knot => "kn",
            Unit::Pa => "pa",
            Unit::KPa => "kpa",
            Unit::Bar => "bar",
            Unit::Atm => "atm",
            Unit::Psi => "psi",
            Unit::Torr => "torr",
            Unit::J => "j",
            Unit::KJ => "kj",
            Unit::Cal => "cal",
            Unit::Kcal => "kcal",
            Unit::Wh => "wh",
            Unit::KWh => "kwh",
            Unit::Ev => "ev",
            Unit::W => "w",
            Unit::KW => "kw",
            Unit::Hp => "hp",
            Unit::N => "n",
            Unit::KN => "knf",
            Unit::Dyn => "dyn",
            Unit::Kgf => "kgf",
            Unit::Lbf => "lbf",
            Unit::Rad => "rad",
            Unit::Deg => "deg",
            Unit::Grad => "grad",
            Unit::ArcMin => "arcmin",
            Unit::ArcSec => "arcsec",
            Unit::Hz => "hz",
            Unit::KHz => "khz",
            Unit::MHz => "mhz",
            Unit::GHz => "ghz",
        }
    }

    fn long_tr(self) -> &'static str {
        match self {
            Unit::Mm => "milimetre",
            Unit::Cm => "santimetre",
            Unit::M => "metre",
            Unit::Km => "kilometre",
            Unit::Inch => "inç",
            Unit::Ft => "fit",
            Unit::Yard => "yard",
            Unit::Mile => "mil",
            Unit::Mg => "miligram",
            Unit::G => "gram",
            Unit::Kg => "kilogram",
            Unit::Ton => "ton",
            Unit::Oz => "ons",
            Unit::Lb => "libre",
            Unit::C => "celsius",
            Unit::F => "fahrenheit",
            Unit::K => "kelvin",
            Unit::B => "byte",
            Unit::KB => "kilobyte",
            Unit::MB => "megabyte",
            Unit::GB => "gigabyte",
            Unit::TB => "terabyte",
            Unit::KiB => "kibibyte",
            Unit::MiB => "mebibyte",
            Unit::GiB => "gibibyte",
            Unit::TiB => "tebibyte",
            Unit::M2 => "metrekare",
            Unit::Km2 => "kilometrekare",
            Unit::Cm2 => "santimetrekare",
            Unit::Mm2 => "milimetrekare",
            Unit::Ha => "hektar",
            Unit::Acre => "acre",
            Unit::Ft2 => "fitkare",
            Unit::In2 => "inçkare",
            Unit::Ml => "mililitre",
            Unit::L => "litre",
            Unit::M3 => "metreküp",
            Unit::Cm3 => "santimetreküp",
            Unit::Ft3 => "fitküp",
            Unit::Gal => "galon",
            Unit::Ns => "nanosaniye",
            Unit::Us => "mikrosaniye",
            Unit::Ms => "milisaniye",
            Unit::S => "saniye",
            Unit::Min => "dakika",
            Unit::H => "saat",
            Unit::Day => "gün",
            Unit::Week => "hafta",
            Unit::Mps => "metre/saniye",
            Unit::Kph => "kilometre/saat",
            Unit::Mph => "mil/saat",
            Unit::Knot => "deniz mili/saat",
            Unit::Pa => "pascal",
            Unit::KPa => "kilopascal",
            Unit::Bar => "bar",
            Unit::Atm => "atmosfer",
            Unit::Psi => "psi",
            Unit::Torr => "torr",
            Unit::J => "joule",
            Unit::KJ => "kilojoule",
            Unit::Cal => "kalori",
            Unit::Kcal => "kilokalori",
            Unit::Wh => "watt-saat",
            Unit::KWh => "kilowatt-saat",
            Unit::Ev => "elektronvolt",
            Unit::W => "watt",
            Unit::KW => "kilowatt",
            Unit::Hp => "beygir gücü",
            Unit::N => "newton",
            Unit::KN => "kilonewton",
            Unit::Dyn => "dyne",
            Unit::Kgf => "kilogram-kuvvet",
            Unit::Lbf => "pound-kuvvet",
            Unit::Rad => "radyan",
            Unit::Deg => "derece",
            Unit::Grad => "grad",
            Unit::ArcMin => "yay dakikası",
            Unit::ArcSec => "yay saniyesi",
            Unit::Hz => "hertz",
            Unit::KHz => "kilohertz",
            Unit::MHz => "megahertz",
            Unit::GHz => "gigahertz",
        }
    }
}

fn parse_unit(s: &str) -> Result<Unit, AppError> {
    let u = s.trim().to_ascii_lowercase();
    let unit = match u.as_str() {
        // length
        "mm" => Unit::Mm,
        "cm" => Unit::Cm,
        "m" | "meter" | "metre" => Unit::M,
        "km" => Unit::Km,
        "in" | "inch" => Unit::Inch,
        "ft" | "feet" => Unit::Ft,
        "yd" | "yard" => Unit::Yard,
        "mi" | "mile" => Unit::Mile,
        // weight
        "mg" => Unit::Mg,
        "g" | "gr" => Unit::G,
        "kg" => Unit::Kg,
        "ton" => Unit::Ton,
        "oz" => Unit::Oz,
        "lb" | "lbs" => Unit::Lb,
        // temperature
        // "celcius" yaygın bir yazım hatası; geriye uyumluluk için kabul ediyoruz.
        "c" | "celcius" | "celsius" => Unit::C,
        "f" | "fahrenheit" => Unit::F,
        "k" | "kelvin" => Unit::K,
        // data (decimal)
        "b" | "byte" => Unit::B,
        "kb" => Unit::KB,
        "mb" => Unit::MB,
        "gb" => Unit::GB,
        "tb" => Unit::TB,
        // data (binary)
        "kib" => Unit::KiB,
        "mib" => Unit::MiB,
        "gib" => Unit::GiB,
        "tib" => Unit::TiB,
        // area
        "m2" | "m^2" => Unit::M2,
        "km2" | "km^2" => Unit::Km2,
        "cm2" | "cm^2" => Unit::Cm2,
        "mm2" | "mm^2" => Unit::Mm2,
        "ha" | "hectare" | "hektar" => Unit::Ha,
        "acre" => Unit::Acre,
        "ft2" | "ft^2" => Unit::Ft2,
        "in2" | "in^2" => Unit::In2,
        // volume
        "ml" => Unit::Ml,
        "l" | "lt" | "liter" | "litre" => Unit::L,
        "m3" | "m^3" => Unit::M3,
        "cm3" | "cm^3" | "cc" => Unit::Cm3,
        "ft3" | "ft^3" => Unit::Ft3,
        "gal" | "gallon" => Unit::Gal,
        // time
        "ns" => Unit::Ns,
        "us" | "µs" => Unit::Us,
        "ms" => Unit::Ms,
        "s" | "sec" | "second" => Unit::S,
        "min" | "minute" => Unit::Min,
        "h" | "hr" | "hour" => Unit::H,
        "day" | "d" => Unit::Day,
        "week" | "wk" => Unit::Week,
        // speed
        "mps" | "m/s" => Unit::Mps,
        "kph" | "km/h" | "kmh" => Unit::Kph,
        "mph" => Unit::Mph,
        "knot" | "kn" => Unit::Knot,
        // pressure
        "pa" => Unit::Pa,
        "kpa" => Unit::KPa,
        "bar" => Unit::Bar,
        "atm" => Unit::Atm,
        "psi" => Unit::Psi,
        "torr" | "mmhg" => Unit::Torr,
        // energy
        "j" | "joule" => Unit::J,
        "kj" => Unit::KJ,
        "cal" => Unit::Cal,
        "kcal" => Unit::Kcal,
        "wh" => Unit::Wh,
        "kwh" => Unit::KWh,
        "ev" => Unit::Ev,
        // power
        "w" | "watt" => Unit::W,
        "kw" => Unit::KW,
        "hp" => Unit::Hp,
        // force
        "n" => Unit::N,
        "knf" => Unit::KN,
        "dyn" | "dyne" => Unit::Dyn,
        "kgf" => Unit::Kgf,
        "lbf" => Unit::Lbf,
        // angle
        "rad" => Unit::Rad,
        "deg" | "degree" => Unit::Deg,
        "grad" | "gon" => Unit::Grad,
        "arcmin" | "amin" => Unit::ArcMin,
        "arcsec" | "asec" => Unit::ArcSec,
        // frequency
        "hz" => Unit::Hz,
        "khz" => Unit::KHz,
        "mhz" => Unit::MHz,
        "ghz" => Unit::GHz,
        _ => return Err(AppError::UnknownUnit(s.to_string())),
    };
    Ok(unit)
}

fn convert(value: f64, from: Unit, to: Unit) -> Result<f64, AppError> {
    if !value.is_finite() {
        return Err(AppError::InvalidValue("NaN veya sonsuz sayı kabul edilmez"));
    }

    if from.kind() != to.kind() {
        return Err(AppError::IncompatibleUnits(
            from.short().to_string(),
            to.short().to_string(),
        ));
    }

    match from.kind() {
        UnitKind::Length => {
            let m = value * length_factor_to_meter(from)?;
            Ok(m / length_factor_to_meter(to)?)
        }
        UnitKind::Weight => {
            let kg = value * weight_factor_to_kg(from)?;
            Ok(kg / weight_factor_to_kg(to)?)
        }
        UnitKind::Temperature => {
            let c = temp_to_c(from, value)?;
            if c < ABSOLUTE_ZERO_C {
                return Err(AppError::InvalidValue(
                    "sıcaklık mutlak sıfırın altında olamaz",
                ));
            }
            c_to_temp(to, c)
        }
        UnitKind::Data => {
            if value < 0.0 {
                return Err(AppError::InvalidValue("veri boyutu negatif olamaz"));
            }
            let b = value * data_factor_to_bytes(from)?;
            Ok(b / data_factor_to_bytes(to)?)
        }
        UnitKind::Area => convert_by_factor(value, from, to, area_factor_to_m2),
        UnitKind::Volume => {
            if value < 0.0 {
                return Err(AppError::InvalidValue("hacim negatif olamaz"));
            }
            convert_by_factor(value, from, to, volume_factor_to_m3)
        }
        UnitKind::Time => {
            if value < 0.0 {
                return Err(AppError::InvalidValue("zaman negatif olamaz"));
            }
            convert_by_factor(value, from, to, time_factor_to_seconds)
        }
        UnitKind::Speed => convert_by_factor(value, from, to, speed_factor_to_mps),
        UnitKind::Pressure => convert_by_factor(value, from, to, pressure_factor_to_pa),
        UnitKind::Energy => convert_by_factor(value, from, to, energy_factor_to_joule),
        UnitKind::Power => convert_by_factor(value, from, to, power_factor_to_watt),
        UnitKind::Force => convert_by_factor(value, from, to, force_factor_to_newton),
        UnitKind::Angle => convert_by_factor(value, from, to, angle_factor_to_radian),
        UnitKind::Frequency => {
            if value < 0.0 {
                return Err(AppError::InvalidValue("frekans negatif olamaz"));
            }
            convert_by_factor(value, from, to, frequency_factor_to_hz)
        }
    }
}

fn convert_by_factor(
    value: f64,
    from: Unit,
    to: Unit,
    factor: fn(Unit) -> Result<f64, AppError>,
) -> Result<f64, AppError> {
    let base = value * factor(from)?;
    Ok(base / factor(to)?)
}

fn length_factor_to_meter(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::Mm => Ok(0.001),
        Unit::Cm => Ok(0.01),
        Unit::M => Ok(1.0),
        Unit::Km => Ok(1000.0),
        Unit::Inch => Ok(0.0254),
        Unit::Ft => Ok(0.3048),
        Unit::Yard => Ok(0.9144),
        Unit::Mile => Ok(1609.344),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "length".to_string(),
        )),
    }
}

fn weight_factor_to_kg(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::Mg => Ok(0.000_001),
        Unit::G => Ok(0.001),
        Unit::Kg => Ok(1.0),
        Unit::Ton => Ok(1000.0),
        Unit::Oz => Ok(0.028_349_523_125),
        Unit::Lb => Ok(0.453_592_37),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "weight".to_string(),
        )),
    }
}

fn temp_to_c(unit: Unit, value: f64) -> Result<f64, AppError> {
    match unit {
        Unit::C => Ok(value),
        Unit::F => Ok((value - 32.0) * (5.0 / 9.0)),
        Unit::K => {
            if value < 0.0 {
                Err(AppError::InvalidValue("Kelvin 0'dan küçük olamaz"))
            } else {
                Ok(value + ABSOLUTE_ZERO_C)
            }
        }
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "temperature".to_string(),
        )),
    }
}

fn c_to_temp(unit: Unit, c: f64) -> Result<f64, AppError> {
    match unit {
        Unit::C => Ok(c),
        Unit::F => Ok(c * (9.0 / 5.0) + 32.0),
        Unit::K => Ok(c - ABSOLUTE_ZERO_C),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "temperature".to_string(),
        )),
    }
}

fn data_factor_to_bytes(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::B => Ok(1.0),
        Unit::KB => Ok(1_000.0),
        Unit::MB => Ok(1_000_000.0),
        Unit::GB => Ok(1_000_000_000.0),
        Unit::TB => Ok(1_000_000_000_000.0),
        Unit::KiB => Ok(KIB),
        Unit::MiB => Ok(MIB),
        Unit::GiB => Ok(GIB),
        Unit::TiB => Ok(TIB),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "data".to_string(),
        )),
    }
}

fn area_factor_to_m2(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::Mm2 => Ok(0.000_001),
        Unit::Cm2 => Ok(0.000_1),
        Unit::M2 => Ok(1.0),
        Unit::Km2 => Ok(1_000_000.0),
        Unit::Ha => Ok(10_000.0),
        Unit::Acre => Ok(4_046.856_422_4),
        Unit::Ft2 => Ok(0.092_903_04),
        Unit::In2 => Ok(0.000_645_16),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "area".to_string(),
        )),
    }
}

fn volume_factor_to_m3(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::Ml | Unit::Cm3 => Ok(0.000_001),
        Unit::L => Ok(0.001),
        Unit::M3 => Ok(1.0),
        Unit::Ft3 => Ok(0.028_316_846_592),
        Unit::Gal => Ok(0.003_785_411_784),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "volume".to_string(),
        )),
    }
}

fn time_factor_to_seconds(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::Ns => Ok(0.000_000_001),
        Unit::Us => Ok(0.000_001),
        Unit::Ms => Ok(0.001),
        Unit::S => Ok(1.0),
        Unit::Min => Ok(60.0),
        Unit::H => Ok(3_600.0),
        Unit::Day => Ok(86_400.0),
        Unit::Week => Ok(604_800.0),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "time".to_string(),
        )),
    }
}

fn speed_factor_to_mps(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::Mps => Ok(1.0),
        Unit::Kph => Ok(1000.0 / 3600.0),
        Unit::Mph => Ok(0.447_04),
        Unit::Knot => Ok(0.514_444_444_444),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "speed".to_string(),
        )),
    }
}

fn pressure_factor_to_pa(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::Pa => Ok(1.0),
        Unit::KPa => Ok(1_000.0),
        Unit::Bar => Ok(100_000.0),
        Unit::Atm => Ok(101_325.0),
        Unit::Psi => Ok(6_894.757_293_168),
        Unit::Torr => Ok(133.322_368_421),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "pressure".to_string(),
        )),
    }
}

fn energy_factor_to_joule(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::J => Ok(1.0),
        Unit::KJ => Ok(1_000.0),
        Unit::Cal => Ok(4.184),
        Unit::Kcal => Ok(4_184.0),
        Unit::Wh => Ok(3_600.0),
        Unit::KWh => Ok(3_600_000.0),
        Unit::Ev => Ok(1.602_176_634e-19),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "energy".to_string(),
        )),
    }
}

fn power_factor_to_watt(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::W => Ok(1.0),
        Unit::KW => Ok(1_000.0),
        Unit::Hp => Ok(745.699_871_582),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "power".to_string(),
        )),
    }
}

fn force_factor_to_newton(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::N => Ok(1.0),
        Unit::KN => Ok(1_000.0),
        Unit::Dyn => Ok(0.000_01),
        Unit::Kgf => Ok(9.806_65),
        Unit::Lbf => Ok(4.448_221_615_260_5),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "force".to_string(),
        )),
    }
}

fn angle_factor_to_radian(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::Rad => Ok(1.0),
        Unit::Deg => Ok(std::f64::consts::PI / 180.0),
        Unit::Grad => Ok(std::f64::consts::PI / 200.0),
        Unit::ArcMin => Ok(std::f64::consts::PI / 10_800.0),
        Unit::ArcSec => Ok(std::f64::consts::PI / 648_000.0),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "angle".to_string(),
        )),
    }
}

fn frequency_factor_to_hz(unit: Unit) -> Result<f64, AppError> {
    match unit {
        Unit::Hz => Ok(1.0),
        Unit::KHz => Ok(1_000.0),
        Unit::MHz => Ok(1_000_000.0),
        Unit::GHz => Ok(1_000_000_000.0),
        _ => Err(AppError::IncompatibleUnits(
            unit.short().to_string(),
            "frequency".to_string(),
        )),
    }
}

#[derive(Debug, Deserialize)]
struct FrankfurterResponse {
    amount: f64,
    base: String,
    rates: HashMap<String, f64>,
}

fn looks_like_currency(code: &str) -> bool {
    let c = code.trim();
    c.len() == 3 && c.chars().all(|ch| ch.is_ascii_alphabetic())
}

fn convert_currency(value: f64, from: &str, to: &str) -> Result<f64, AppError> {
    if !value.is_finite() {
        return Err(AppError::InvalidValue("NaN veya sonsuz sayı kabul edilmez"));
    }

    let from_up = from.trim().to_ascii_uppercase();
    let to_up = to.trim().to_ascii_uppercase();

    if !looks_like_currency(&from_up) || !looks_like_currency(&to_up) {
        return Err(AppError::UnknownUnit(format!("{from} veya {to}")));
    }

    if from_up == to_up {
        return Ok(value);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| AppError::Network(e.to_string()))?;

    let resp = client
        .get(format!(
            "{FRANKFURTER_LATEST_URL}?amount={value}&from={from_up}&to={to_up}"
        ))
        .send()
        .map_err(|e| AppError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(AppError::Api(format!("HTTP durum kodu: {}", resp.status())));
    }

    let parsed: FrankfurterResponse = resp.json().map_err(|e| AppError::Api(e.to_string()))?;

    if !parsed.base.eq_ignore_ascii_case(&from_up) {
        return Err(AppError::Api("beklenmeyen base değeri döndü".to_string()));
    }
    if (parsed.amount - value).abs() > 1e-9 {
        return Err(AppError::Api("beklenmeyen amount değeri döndü".to_string()));
    }

    parsed
        .rates
        .get(&to_up)
        .copied()
        .ok_or_else(|| AppError::Api(format!("{to_up} kuru bulunamadı")))
}

fn run(cli: Cli) -> Result<String, AppError> {
    // Önce yerel birim dönüşümünü dene.
    match (parse_unit(&cli.from), parse_unit(&cli.to)) {
        (Ok(from_u), Ok(to_u)) => {
            let out = convert(cli.value, from_u, to_u)?;
            Ok(format!(
                "{} {} ({}) = {:.6} {} ({})",
                cli.value,
                from_u.long_tr(),
                from_u.short(),
                out,
                to_u.long_tr(),
                to_u.short()
            ))
        }
        (Err(_), Err(_)) => {
            // İkisi de yerel birim değilse döviz varsayımıyla API çağrısı yap.
            let out = convert_currency(cli.value, &cli.from, &cli.to)?;
            Ok(format!(
                "{:.6} {} = {:.6} {} (Frankfurter)",
                cli.value,
                cli.from.to_ascii_uppercase(),
                out,
                cli.to.to_ascii_uppercase()
            ))
        }
        (Ok(from_u), Err(_)) => Err(AppError::IncompatibleUnits(
            from_u.short().to_string(),
            cli.to.clone(),
        )),
        (Err(_), Ok(to_u)) => Err(AppError::IncompatibleUnits(
            cli.from.clone(),
            to_u.short().to_string(),
        )),
    }
}

fn main() {
    let cli = Cli::parse();
    match run(cli) {
        Ok(output) => println!("{output}"),
        Err(e) => {
            eprintln!("Hata: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_units() {
        assert!(matches!(parse_unit("km"), Ok(Unit::Km)));
        assert!(matches!(parse_unit("mib"), Ok(Unit::MiB)));
        assert!(parse_unit("t").is_err());
        assert!(parse_unit("unknown").is_err());
    }

    #[test]
    fn parses_negative_value() {
        let cli =
            Cli::try_parse_from(["uco", "-40", "c", "f"]).expect("negatif değer kabul edilmeli");

        assert_eq!(cli.value, -40.0);
        assert_eq!(cli.from, "c");
        assert_eq!(cli.to, "f");
    }

    #[test]
    fn convert_km_to_m() {
        let out = convert(5.0, Unit::Km, Unit::M).expect("ok");
        assert!((out - 5000.0).abs() < 1e-9);
    }

    #[test]
    fn convert_lb_to_kg() {
        let out = convert(10.0, Unit::Lb, Unit::Kg).expect("ok");
        assert!((out - 4.535_923_7).abs() < 1e-9);
    }

    #[test]
    fn convert_temperature() {
        let out = convert(32.0, Unit::F, Unit::C).expect("ok");
        assert!(out.abs() < 1e-12);
    }

    #[test]
    fn rejects_temperature_below_absolute_zero() {
        assert!(matches!(
            convert(-300.0, Unit::C, Unit::K),
            Err(AppError::InvalidValue(_))
        ));
        assert!(matches!(
            convert(-500.0, Unit::F, Unit::C),
            Err(AppError::InvalidValue(_))
        ));
    }

    #[test]
    fn incompatible_units_rejected() {
        assert!(matches!(
            convert(5.0, Unit::Km, Unit::Kg),
            Err(AppError::IncompatibleUnits(_, _))
        ));
    }

    #[test]
    fn detects_currency_code_shape() {
        assert!(looks_like_currency("usd"));
        assert!(looks_like_currency("TRY"));
        assert!(!looks_like_currency("usdt"));
        assert!(!looks_like_currency("12a"));
    }

    #[test]
    fn parse_engineering_units() {
        assert!(matches!(parse_unit("bar"), Ok(Unit::Bar)));
        assert!(matches!(parse_unit("kwh"), Ok(Unit::KWh)));
        assert!(matches!(parse_unit("m/s"), Ok(Unit::Mps)));
        assert!(matches!(parse_unit("m^2"), Ok(Unit::M2)));
        assert!(matches!(parse_unit("cc"), Ok(Unit::Cm3)));
        assert!(matches!(parse_unit("knf"), Ok(Unit::KN)));
    }

    #[test]
    fn convert_area_volume_time_and_speed() {
        let area = convert(1.0, Unit::Ha, Unit::M2).expect("area ok");
        assert!((area - 10_000.0).abs() < 1e-9);

        let volume = convert(1.0, Unit::L, Unit::Ml).expect("volume ok");
        assert!((volume - 1000.0).abs() < 1e-9);

        let time = convert(2.0, Unit::H, Unit::Min).expect("time ok");
        assert!((time - 120.0).abs() < 1e-9);

        let speed = convert(36.0, Unit::Kph, Unit::Mps).expect("speed ok");
        assert!((speed - 10.0).abs() < 1e-9);
    }

    #[test]
    fn convert_pressure_energy_power_force_angle_frequency() {
        let pressure = convert(1.0, Unit::Bar, Unit::Pa).expect("pressure ok");
        assert!((pressure - 100_000.0).abs() < 1e-9);

        let energy = convert(1.0, Unit::KWh, Unit::J).expect("energy ok");
        assert!((energy - 3_600_000.0).abs() < 1e-9);

        let power = convert(1.0, Unit::KW, Unit::W).expect("power ok");
        assert!((power - 1000.0).abs() < 1e-9);

        let force = convert(1.0, Unit::Kgf, Unit::N).expect("force ok");
        assert!((force - 9.806_65).abs() < 1e-9);

        let angle = convert(180.0, Unit::Deg, Unit::Rad).expect("angle ok");
        assert!((angle - std::f64::consts::PI).abs() < 1e-12);

        let frequency = convert(2.0, Unit::GHz, Unit::MHz).expect("frequency ok");
        assert!((frequency - 2000.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_negative_physical_quantities_where_needed() {
        assert!(matches!(
            convert(-1.0, Unit::L, Unit::Ml),
            Err(AppError::InvalidValue(_))
        ));
        assert!(matches!(
            convert(-1.0, Unit::S, Unit::Ms),
            Err(AppError::InvalidValue(_))
        ));
        assert!(matches!(
            convert(-1.0, Unit::Hz, Unit::KHz),
            Err(AppError::InvalidValue(_))
        ));
    }
}
