// Basit kullanım odaklı birim + döviz dönüştürücü:
// örnek: uco 5 km m
// örnek: uco 100 usd try
// Güvenli yaklaşım: tek sorumluluklu fonksiyonlar, doğrulama, anlamlı hata mesajları.

use serde::Deserialize;
use std::env;

const ABSOLUTE_ZERO_C: f64 = -273.15;

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
            format!("{from:?}"),
            format!("{to:?}"),
        ));
    }

    match from.kind() {
        UnitKind::Length => {
            let m = length_to_meter(from, value);
            Ok(meter_to_length(to, m))
        }
        UnitKind::Weight => {
            let kg = weight_to_kg(from, value);
            Ok(kg_to_weight(to, kg))
        }
        UnitKind::Temperature => {
            let c = temp_to_c(from, value)?;
            if c < ABSOLUTE_ZERO_C {
                return Err(AppError::InvalidValue(
                    "sıcaklık mutlak sıfırın altında olamaz",
                ));
            }
            Ok(c_to_temp(to, c))
        }
        UnitKind::Data => {
            if value < 0.0 {
                return Err(AppError::InvalidValue("veri boyutu negatif olamaz"));
            }
            let b = data_to_bytes(from, value);
            Ok(bytes_to_data(to, b))
        }
    }
}

fn length_to_meter(unit: Unit, value: f64) -> f64 {
    match unit {
        Unit::Mm => value / 1000.0,
        Unit::Cm => value / 100.0,
        Unit::M => value,
        Unit::Km => value * 1000.0,
        Unit::Inch => value * 0.0254,
        Unit::Ft => value * 0.3048,
        Unit::Yard => value * 0.9144,
        Unit::Mile => value * 1609.344,
        _ => unreachable!("length unit bekleniyordu"),
    }
}

fn meter_to_length(unit: Unit, meter: f64) -> f64 {
    match unit {
        Unit::Mm => meter * 1000.0,
        Unit::Cm => meter * 100.0,
        Unit::M => meter,
        Unit::Km => meter / 1000.0,
        Unit::Inch => meter / 0.0254,
        Unit::Ft => meter / 0.3048,
        Unit::Yard => meter / 0.9144,
        Unit::Mile => meter / 1609.344,
        _ => unreachable!("length unit bekleniyordu"),
    }
}

fn weight_to_kg(unit: Unit, value: f64) -> f64 {
    match unit {
        Unit::Mg => value / 1_000_000.0,
        Unit::G => value / 1000.0,
        Unit::Kg => value,
        Unit::Ton => value * 1000.0,
        Unit::Oz => value * 0.028_349_523_125,
        Unit::Lb => value * 0.453_592_37,
        _ => unreachable!("weight unit bekleniyordu"),
    }
}

fn kg_to_weight(unit: Unit, kg: f64) -> f64 {
    match unit {
        Unit::Mg => kg * 1_000_000.0,
        Unit::G => kg * 1000.0,
        Unit::Kg => kg,
        Unit::Ton => kg / 1000.0,
        Unit::Oz => kg / 0.028_349_523_125,
        Unit::Lb => kg / 0.453_592_37,
        _ => unreachable!("weight unit bekleniyordu"),
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
        _ => unreachable!("temperature unit bekleniyordu"),
    }
}

fn c_to_temp(unit: Unit, c: f64) -> f64 {
    match unit {
        Unit::C => c,
        Unit::F => c * (9.0 / 5.0) + 32.0,
        Unit::K => c + 273.15,
        _ => unreachable!("temperature unit bekleniyordu"),
    }
}

fn data_to_bytes(unit: Unit, value: f64) -> f64 {
    match unit {
        Unit::B => value,
        Unit::KB => value * 1_000.0,
        Unit::MB => value * 1_000_000.0,
        Unit::GB => value * 1_000_000_000.0,
        Unit::TB => value * 1_000_000_000_000.0,
        Unit::KiB => value * 1024.0,
        Unit::MiB => value * 1024.0_f64.powi(2),
        Unit::GiB => value * 1024.0_f64.powi(3),
        Unit::TiB => value * 1024.0_f64.powi(4),
        _ => unreachable!("data unit bekleniyordu"),
    }
}

fn bytes_to_data(unit: Unit, bytes: f64) -> f64 {
    match unit {
        Unit::B => bytes,
        Unit::KB => bytes / 1_000.0,
        Unit::MB => bytes / 1_000_000.0,
        Unit::GB => bytes / 1_000_000_000.0,
        Unit::TB => bytes / 1_000_000_000_000.0,
        Unit::KiB => bytes / 1024.0,
        Unit::MiB => bytes / 1024.0_f64.powi(2),
        Unit::GiB => bytes / 1024.0_f64.powi(3),
        Unit::TiB => bytes / 1024.0_f64.powi(4),
        _ => unreachable!("data unit bekleniyordu"),
    }
}

#[derive(Debug, Deserialize)]
struct FrankfurterResponse {
    amount: f64,
    base: String,
    rates: std::collections::HashMap<String, f64>,
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

    let url = format!(
        "https://api.frankfurter.app/latest?amount={}&from={}&to={}",
        value, from_up, to_up
    );

    let resp = reqwest::blocking::get(&url).map_err(|e| AppError::Network(e.to_string()))?;

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
}
