// Bu binary, Windows/Linux/macOS üzerinde çevredeki Wi-Fi ağlarını tarar.
// Güvenlik odağı: sadece SSID ve güvenlik tipini okur, şifre denemez/toplamaz.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::thread;
#[cfg(target_os = "windows")]
use std::time::Duration;

const UNKNOWN_SECURITY: &str = "Unknown";

// Windows Native Wi-Fi API'deki WlanScan asenkron çalışır. Bu yüzden scan çağrısından
// hemen sonra netsh çıktısını okumak bazen eski sonuçları verebilir. Çok uzun beklemek
// programı yavaşlatır; pratikte 2 saniye çoğu adaptörde yeterli ve 4 saniyeden hızlıdır.
#[cfg(target_os = "windows")]
const WINDOWS_SCAN_SETTLE_TIME: Duration = Duration::from_secs(2);

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NetworkDetails {
    encryption: Option<String>,
    signal: Option<u8>,
    radio_type: Option<String>,
    band: Option<String>,
    channel: Option<String>,
    connected_stations: Option<u32>,
    channel_utilization: Option<String>,
    basic_rates: Option<String>,
    other_rates: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WifiNetwork {
    ssid: String,
    bssid: Option<String>,
    authentication: String,
    is_open: bool,
    details: NetworkDetails,
}

impl WifiNetwork {
    #[cfg(test)]
    fn new(
        ssid: impl Into<String>,
        bssid: Option<impl Into<String>>,
        authentication: impl Into<String>,
    ) -> Self {
        Self::with_details(ssid, bssid, authentication, NetworkDetails::default())
    }

    fn with_details(
        ssid: impl Into<String>,
        bssid: Option<impl Into<String>>,
        authentication: impl Into<String>,
        details: NetworkDetails,
    ) -> Self {
        let authentication = normalize_security(authentication.into());

        Self {
            ssid: ssid.into(),
            bssid: bssid.map(Into::into),
            is_open: is_open_auth(&authentication),
            authentication,
            details,
        }
    }
}

#[derive(Debug)]
enum ScanError {
    CommandFailed(String),
    ParseError(String),
    NoNetworksVisible,
    NoWirelessInterface,
    UnsupportedPlatform(String),
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandFailed(msg) => write!(f, "komut hatası: {msg}"),
            Self::ParseError(msg) => write!(f, "ayrıştırma hatası: {msg}"),
            Self::NoNetworksVisible => write!(f, "çevrede görünür Wi-Fi ağı bulunamadı"),
            Self::NoWirelessInterface => write!(f, "sistemde kablosuz ağ arayüzü bulunamadı"),
            Self::UnsupportedPlatform(msg) => write!(f, "desteklenmeyen platform: {msg}"),
        }
    }
}

impl std::error::Error for ScanError {}

impl ScanError {
    #[must_use]
    fn user_hint(&self) -> Option<&'static str> {
        match self {
            Self::CommandFailed(msg)
                if contains_any_lower(
                    msg,
                    &[
                        "wireless autoconfig service (wlansvc) is not running",
                        "wlansvc service is not running",
                    ],
                ) =>
            {
                Some(
                    "wlansvc servisi kapalı. Yönetici PowerShell/CMD ile şu komutu çalıştır: `sc start wlansvc`",
                )
            }
            Self::NoWirelessInterface => Some(
                "Wi-Fi adaptörü kapalı/eksik olabilir. Aygıt Yöneticisi'nden kablosuz adaptörü kontrol et.",
            ),
            Self::CommandFailed(msg)
                if contains_any_lower(
                    msg,
                    &[
                        "location permission",
                        "requires elevation",
                        "konum izni",
                        "yönetici",
                    ],
                ) =>
            {
                Some(
                    "Windows WLAN taraması için Konum Servisleri açık olmalı; gerekirse programı yönetici olarak çalıştır.",
                )
            }
            Self::CommandFailed(msg)
                if contains_any_lower(msg, &["no such file or directory", "not found"]) =>
            {
                Some("gerekli sistem aracı bulunamadı (netsh/nmcli/airport).")
            }
            _ => None,
        }
    }
}

fn contains_any_lower(value: &str, needles: &[&str]) -> bool {
    let lower = value.to_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn normalize_security(security: String) -> String {
    let trimmed = security.trim();
    if trimmed.is_empty() {
        UNKNOWN_SECURITY.to_string()
    } else {
        trimmed.to_string()
    }
}

fn is_open_auth(auth: &str) -> bool {
    let normalized = auth.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "open" | "none" | "--" | "açık" | "acik"
    )
}

fn add_network(
    networks: &mut BTreeMap<(String, Option<String>), WifiNetwork>,
    ssid: impl Into<String>,
    bssid: Option<impl Into<String>>,
    authentication: impl Into<String>,
) {
    add_network_with_details(
        networks,
        ssid,
        bssid,
        authentication,
        NetworkDetails::default(),
    );
}

fn add_network_with_details(
    networks: &mut BTreeMap<(String, Option<String>), WifiNetwork>,
    ssid: impl Into<String>,
    bssid: Option<impl Into<String>>,
    authentication: impl Into<String>,
    details: NetworkDetails,
) {
    let ssid = ssid.into().trim().to_string();
    if ssid.is_empty() {
        return;
    }

    let bssid = bssid
        .map(Into::into)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    // Tekilleştirme anahtarı `(SSID, BSSID)`:
    // - Aynı SSID'nin 2.4/5/6 GHz gibi birden fazla erişim noktası olabilir.
    // - BSSID varsa her erişim noktasını ayrı tutuyoruz.
    // - Raporlama aşamasında bunları tekrar SSID altında grupluyoruz.
    let candidate = WifiNetwork::with_details(ssid.clone(), bssid.clone(), authentication, details);
    networks
        .entry((ssid, bssid))
        .and_modify(|existing| {
            if !existing
                .authentication
                .split(", ")
                .any(|part| part == candidate.authentication)
            {
                existing.authentication.push_str(", ");
                existing.authentication.push_str(&candidate.authentication);
            }
            existing.is_open |= candidate.is_open;
            merge_details(&mut existing.details, &candidate.details);
        })
        .or_insert(candidate);
}

fn merge_details(existing: &mut NetworkDetails, candidate: &NetworkDetails) {
    // Aynı ağ tekrar görülürse boş alanları doldur, sinyal için en güçlü değeri koru.
    // Bu yaklaşım parser'ı toleranslı yapar: Bazı platformlar/driver'lar her alanı döndürmeyebilir.
    existing.encryption = existing
        .encryption
        .clone()
        .or_else(|| candidate.encryption.clone());
    existing.signal = existing.signal.max(candidate.signal);
    existing.radio_type = existing
        .radio_type
        .clone()
        .or_else(|| candidate.radio_type.clone());
    existing.band = existing.band.clone().or_else(|| candidate.band.clone());
    existing.channel = existing
        .channel
        .clone()
        .or_else(|| candidate.channel.clone());
    existing.connected_stations = existing.connected_stations.or(candidate.connected_stations);
    existing.channel_utilization = existing
        .channel_utilization
        .clone()
        .or_else(|| candidate.channel_utilization.clone());
    existing.basic_rates = existing
        .basic_rates
        .clone()
        .or_else(|| candidate.basic_rates.clone());
    existing.other_rates = existing
        .other_rates
        .clone()
        .or_else(|| candidate.other_rates.clone());
}

fn into_sorted_networks(
    networks: BTreeMap<(String, Option<String>), WifiNetwork>,
) -> Vec<WifiNetwork> {
    networks.into_values().collect()
}

fn line_value_for_labels<'a>(line: &'a str, labels: &[&str]) -> Option<&'a str> {
    // netsh çıktısı yerel dile göre değişebilir. Örneğin Windows Türkçe ise
    // "Authentication" yerine "Kimlik Doğrulaması" gelebilir. Bu yüzden
    // alan adlarını küçük harfe çevirip birden fazla etiketle eşleştiriyoruz.
    let (key, value) = line.split_once(':')?;
    let normalized_key = key.trim().to_lowercase();

    labels
        .iter()
        .any(|label| normalized_key == *label)
        .then_some(value.trim())
}

fn windows_ssid_value(line: &str) -> Option<&str> {
    let (key, value) = line.split_once(':')?;
    let key = key.trim();
    let suffix = key.strip_prefix("SSID ")?;

    suffix
        .trim()
        .chars()
        .all(|character| character.is_ascii_digit())
        .then_some(value.trim())
}

fn windows_bssid_value(line: &str) -> Option<&str> {
    let (key, value) = line.split_once(':')?;
    let key = key.trim();
    let suffix = key.strip_prefix("BSSID ")?;

    suffix
        .trim()
        .chars()
        .all(|character| character.is_ascii_digit())
        .then_some(value.trim())
}

#[derive(Debug, Clone)]
struct WindowsBssid {
    bssid: String,
    details: NetworkDetails,
}

fn parse_percentage(value: &str) -> Option<u8> {
    value
        .trim()
        .trim_end_matches('%')
        .trim()
        .parse::<u8>()
        .ok()
        .map(|percentage| percentage.min(100))
}

fn parse_first_u32(value: &str) -> Option<u32> {
    value
        .split(|character: char| !character.is_ascii_digit())
        .find(|part| !part.is_empty())
        .and_then(|part| part.parse::<u32>().ok())
}

fn add_windows_group(
    networks: &mut BTreeMap<(String, Option<String>), WifiNetwork>,
    ssid: String,
    bssids: Vec<WindowsBssid>,
    authentication: Option<String>,
    group_details: NetworkDetails,
) {
    let authentication = authentication.unwrap_or_else(|| UNKNOWN_SECURITY.to_string());

    if bssids.is_empty() {
        add_network_with_details(
            networks,
            ssid,
            None::<String>,
            authentication,
            group_details,
        );
        return;
    }

    for bssid in bssids {
        let mut details = group_details.clone();
        merge_details(&mut details, &bssid.details);
        add_network_with_details(
            networks,
            ssid.clone(),
            Some(bssid.bssid),
            authentication.clone(),
            details,
        );
    }
}

#[must_use]
fn parse_windows_netsh(output: &str) -> Vec<WifiNetwork> {
    let mut networks = BTreeMap::new();
    let mut current_ssid: Option<String> = None;
    let mut current_auth: Option<String> = None;
    let mut current_group_details = NetworkDetails::default();
    let mut current_bssids: Vec<WindowsBssid> = Vec::new();
    let mut current_bssid_details: Option<WindowsBssid> = None;

    for raw_line in output.lines() {
        let line = raw_line.trim();

        if let Some(ssid) = windows_ssid_value(line) {
            // Yeni SSID başladıysa önce önceki SSID'nin son BSSID bloğunu kapat.
            if let Some(bssid) = current_bssid_details.take() {
                current_bssids.push(bssid);
            }

            // Sonra önceki SSID grubunu ana haritaya ekle.
            if let Some(previous_ssid) = current_ssid.take() {
                add_windows_group(
                    &mut networks,
                    previous_ssid,
                    std::mem::take(&mut current_bssids),
                    current_auth.take(),
                    std::mem::take(&mut current_group_details),
                );
            }

            current_ssid = (!ssid.is_empty()).then(|| ssid.to_string());
            current_auth = None;
            current_group_details = NetworkDetails::default();
            continue;
        }

        if let Some(authentication) = line_value_for_labels(
            line,
            &["authentication", "kimlik doğrulaması", "kimlik dogrulamasi"],
        ) {
            if !authentication.is_empty() {
                current_auth = Some(authentication.to_string());
            }
            continue;
        }

        if let Some(encryption) =
            line_value_for_labels(line, &["encryption", "şifreleme", "sifreleme"])
        {
            if !encryption.is_empty() {
                current_group_details.encryption = Some(encryption.to_string());
            }
            continue;
        }

        if let Some(bssid) = windows_bssid_value(line)
            && !bssid.is_empty()
        {
            // Her BSSID altında sinyal/band/kanal gibi kendi alt alanları gelir.
            // Yeni BSSID görünce önceki BSSID'nin detaylarını listeye taşıyoruz.
            if let Some(previous_bssid) = current_bssid_details.take() {
                current_bssids.push(previous_bssid);
            }
            current_bssid_details = Some(WindowsBssid {
                bssid: bssid.to_string(),
                details: NetworkDetails::default(),
            });
            continue;
        }

        let Some(current_bssid) = current_bssid_details.as_mut() else {
            continue;
        };

        if let Some(signal) = line_value_for_labels(line, &["signal", "sinyal"]) {
            current_bssid.details.signal = parse_percentage(signal);
        } else if let Some(radio_type) =
            line_value_for_labels(line, &["radio type", "radyo türü", "radyo turu"])
        {
            current_bssid.details.radio_type = Some(radio_type.to_string());
        } else if let Some(band) = line_value_for_labels(line, &["band", "bant"]) {
            current_bssid.details.band = Some(band.to_string());
        } else if let Some(channel) = line_value_for_labels(line, &["channel", "kanal"]) {
            current_bssid.details.channel = Some(channel.to_string());
        } else if let Some(stations) = line_value_for_labels(
            line,
            &[
                "connected stations",
                "bağlı istasyonlar",
                "bagli istasyonlar",
            ],
        ) {
            current_bssid.details.connected_stations = parse_first_u32(stations);
        } else if let Some(utilization) = line_value_for_labels(
            line,
            &["channel utilization", "kanal kullanımı", "kanal kullanimi"],
        ) {
            current_bssid.details.channel_utilization = Some(utilization.to_string());
        } else if let Some(basic_rates) = line_value_for_labels(
            line,
            &[
                "basic rates (mbps)",
                "temel hızlar (mbps)",
                "temel hizlar (mbps)",
            ],
        ) {
            current_bssid.details.basic_rates = Some(basic_rates.to_string());
        } else if let Some(other_rates) = line_value_for_labels(
            line,
            &[
                "other rates (mbps)",
                "diğer hızlar (mbps)",
                "diger hizlar (mbps)",
            ],
        ) {
            current_bssid.details.other_rates = Some(other_rates.to_string());
        }
    }

    if let Some(bssid) = current_bssid_details {
        current_bssids.push(bssid);
    }

    if let Some(ssid) = current_ssid {
        add_windows_group(
            &mut networks,
            ssid,
            current_bssids,
            current_auth,
            current_group_details,
        );
    }

    into_sorted_networks(networks)
}

fn split_nmcli_network_record(line: &str) -> Option<WifiNetwork> {
    // nmcli -t çıktısında alan ayırıcı ':' karakteridir; SSID veya BSSID içinde ':'
    // geçerse nmcli bunu '\:' şeklinde escape eder. split_nmcli_fields bu kaçışları çözer.
    let fields = split_nmcli_fields(line);
    match fields.as_slice() {
        [ssid, bssid, security, signal, freq, channel, rate] => {
            let details = NetworkDetails {
                signal: parse_percentage(signal),
                band: linux_band_from_frequency(freq),
                channel: non_empty_string(channel),
                radio_type: linux_radio_type_from_rate(rate),
                other_rates: non_empty_string(rate),
                ..NetworkDetails::default()
            };

            Some(WifiNetwork::with_details(
                ssid.to_string(),
                non_empty_string(bssid),
                security.to_string(),
                details,
            ))
        }
        [ssid, bssid, security] => Some(WifiNetwork::with_details(
            ssid.to_string(),
            non_empty_string(bssid),
            security.to_string(),
            NetworkDetails::default(),
        )),
        [ssid, security] => Some(WifiNetwork::with_details(
            ssid.to_string(),
            None::<String>,
            security.to_string(),
            NetworkDetails::default(),
        )),
        _ => None,
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty() && trimmed != "--").then(|| trimmed.to_string())
}

fn linux_band_from_frequency(freq: &str) -> Option<String> {
    let mhz = parse_first_u32(freq)?;
    match mhz {
        2400..=2500 => Some("2.4 GHz".to_string()),
        4900..=5900 => Some("5 GHz".to_string()),
        5925..=7125 => Some("6 GHz".to_string()),
        _ => Some(format!("{mhz} MHz")),
    }
}

fn linux_radio_type_from_rate(rate: &str) -> Option<String> {
    // nmcli doğrudan 802.11 standardını her sistemde vermeyebilir. RATE alanından
    // yaklaşık bir etiket üretiyoruz. Bu kesin bir ölçüm değil, kullanıcıya yardımcı
    // olacak okunabilir bir tahmindir.
    let mbps = rate
        .split_whitespace()
        .next()
        .and_then(|value| value.parse::<f32>().ok())?;

    let label = if mbps >= 600.0 {
        "802.11ac/ax"
    } else if mbps >= 150.0 {
        "802.11n/ac"
    } else if mbps >= 54.0 {
        "802.11a/g"
    } else {
        "802.11b/g"
    };

    Some(label.to_string())
}

fn normalize_bssid_for_oui(bssid: &str) -> Option<String> {
    let hex: String = bssid
        .chars()
        .filter(|character| character.is_ascii_hexdigit())
        .map(|character| character.to_ascii_uppercase())
        .collect();

    (hex.len() >= 6).then(|| hex[..6].to_string())
}

fn vendor_from_bssid(bssid: &str) -> &'static str {
    let Some(oui) = normalize_bssid_for_oui(bssid) else {
        return "Bilinmiyor";
    };

    // OUI (Organizationally Unique Identifier), MAC/BSSID'nin ilk 24 bitidir.
    // Tam üretici veritabanları büyük dosyalar gerektirir. Bu küçük tablo, yerel
    // çıktılarda sık görülen bazı üreticileri göstermek için bakım dostu bir başlangıçtır.
    // Bilinmeyen OUI'lerde yine BSSID gösterildiği için cihaz kimliği kaybolmaz.
    match oui.as_str() {
        "001A2B" | "30CC21" | "98BA5F" | "9ABA5F" => "Arcadyan / ISP CPE",
        "5478F0" => "ZTE / ISP CPE",
        "DC094C" => "TP-Link / Mercusys",
        "C89828" => "Huawei / ISP CPE",
        "F0B014" | "F4F5D8" => "Xiaomi",
        "A4CF12" | "B0BE76" => "Apple",
        "001B2F" | "D8B190" => "Cisco / Linksys",
        "E894F6" | "FC3497" => "TP-Link",
        "001D0F" | "A42B8C" => "ASUS",
        "B827EB" | "DCA632" => "Raspberry Pi",
        _ => "Bilinmiyor",
    }
}

fn device_identity(bssid: Option<&str>) -> String {
    match bssid {
        Some(value) => format!("{} ({})", vendor_from_bssid(value), value),
        None => "-".to_string(),
    }
}

fn split_nmcli_fields(line: &str) -> Vec<String> {
    // Basit `line.split(':')` kullanmıyoruz; çünkü nmcli `--escape yes` ile
    // gerçek ':' karakterlerini '\:' olarak döndürür. Bu küçük ayrıştırıcı,
    // kaçış karakterini tüketip alanları doğru şekilde ayırır.
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for character in line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            ':' => {
                fields.push(current);
                current = String::new();
            }
            _ => current.push(character),
        }
    }

    if escaped {
        current.push('\\');
    }
    fields.push(current);

    fields
}

#[must_use]
fn parse_linux_nmcli(output: &str) -> Vec<WifiNetwork> {
    let mut networks = BTreeMap::new();

    for line in output.lines() {
        if let Some(network) = split_nmcli_network_record(line) {
            add_network_with_details(
                &mut networks,
                network.ssid,
                network.bssid,
                network.authentication,
                network.details,
            );
        }
    }

    into_sorted_networks(networks)
}

fn is_bssid_token(value: &str) -> bool {
    let groups: Vec<&str> = value.split(':').collect();

    groups.len() == 6
        && groups.iter().all(|group| {
            group.len() == 2 && group.chars().all(|character| character.is_ascii_hexdigit())
        })
}

#[must_use]
fn parse_macos_airport(output: &str) -> Vec<WifiNetwork> {
    let mut networks = BTreeMap::new();

    // airport -s çıktısı: SSID BSSID RSSI CHANNEL HT CC SECURITY
    for (idx, line) in output.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let Some(bssid_idx) = parts.iter().position(|part| is_bssid_token(part)) else {
            continue;
        };

        let ssid = parts[..bssid_idx].join(" ");
        if ssid.trim().is_empty() {
            continue;
        }

        let after_bssid = &parts[bssid_idx + 1..];
        let security = if after_bssid.len() >= 5 {
            after_bssid[4..].join(" ")
        } else {
            after_bssid
                .last()
                .copied()
                .unwrap_or(UNKNOWN_SECURITY)
                .to_string()
        };

        add_network(
            &mut networks,
            ssid,
            Some(parts[bssid_idx].to_string()),
            security,
        );
    }

    into_sorted_networks(networks)
}

fn command_stdout(program: &str, args: &[&str]) -> Result<String, ScanError> {
    // Güvenlik notu: Komutları shell üzerinden değil, Command::new + args ile çalıştırıyoruz.
    // Böylece kullanıcı girdisi olsaydı bile shell injection riski oluşmazdı.
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|err| ScanError::CommandFailed(format!("{program}: {err}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!(
                "{program} başarısız oldu (exit: {:?})",
                output.status.code()
            )
        };
        return Err(ScanError::CommandFailed(detail));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn ensure_networks(raw: &str, networks: Vec<WifiNetwork>) -> Result<Vec<WifiNetwork>, ScanError> {
    if !networks.is_empty() {
        return Ok(networks);
    }

    if raw.trim().is_empty()
        || contains_any_lower(
            raw,
            &[
                "there are no networks currently visible",
                "no networks found",
                "no wifi networks",
            ],
        )
    {
        return Err(ScanError::NoNetworksVisible);
    }

    Err(ScanError::ParseError(format!(
        "çıktı beklenen formatta değil (ilk 120 karakter: {})",
        raw.chars().take(120).collect::<String>()
    )))
}

#[cfg(target_os = "windows")]
fn trigger_windows_wifi_scan() -> Result<(), ScanError> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
    use windows_sys::Win32::NetworkManagement::WiFi::{
        WLAN_INTERFACE_INFO, WLAN_INTERFACE_INFO_LIST, WlanCloseHandle, WlanEnumInterfaces,
        WlanFreeMemory, WlanOpenHandle, WlanScan,
    };

    struct WlanHandle(HANDLE);

    impl Drop for WlanHandle {
        fn drop(&mut self) {
            // SAFETY: Handle, başarılı WlanOpenHandle çağrısından geliyor; reserved parametresi API gereği null.
            unsafe {
                WlanCloseHandle(self.0, null());
            }
        }
    }

    let mut negotiated_version = 0;
    let mut raw_handle: HANDLE = null_mut();

    // SAFETY: Çıktı pointer'ları geçerli yerel değişkenlere işaret ediyor; reserved parametresi API gereği null.
    let open_result =
        unsafe { WlanOpenHandle(2, null(), &mut negotiated_version, &mut raw_handle) };
    if open_result != ERROR_SUCCESS {
        return Err(ScanError::CommandFailed(format!(
            "WlanOpenHandle başarısız oldu (Windows hata kodu: {open_result})"
        )));
    }

    let handle = WlanHandle(raw_handle);
    let mut interface_list: *mut WLAN_INTERFACE_INFO_LIST = null_mut();

    // SAFETY: handle geçerli; interface_list API tarafından ayrılıp WlanFreeMemory ile serbest bırakılıyor.
    let enum_result = unsafe { WlanEnumInterfaces(handle.0, null(), &mut interface_list) };
    if enum_result != ERROR_SUCCESS {
        return Err(ScanError::CommandFailed(format!(
            "WlanEnumInterfaces başarısız oldu (Windows hata kodu: {enum_result})"
        )));
    }

    if interface_list.is_null() {
        return Err(ScanError::NoWirelessInterface);
    }

    struct WlanMemory(*mut WLAN_INTERFACE_INFO_LIST);

    impl Drop for WlanMemory {
        fn drop(&mut self) {
            // SAFETY: Pointer, WlanEnumInterfaces tarafından ayrılan belleğe işaret ediyor.
            unsafe {
                WlanFreeMemory(self.0.cast());
            }
        }
    }

    let interface_list = WlanMemory(interface_list);

    // SAFETY: interface_list null değil ve WLAN_INTERFACE_INFO_LIST düzeninde; esnek dizi için ilk eleman adresinden slice oluşturuluyor.
    let interfaces = unsafe {
        let count = (*interface_list.0).dwNumberOfItems as usize;
        let first = (*interface_list.0).InterfaceInfo.as_ptr();
        std::slice::from_raw_parts::<WLAN_INTERFACE_INFO>(first, count)
    };

    if interfaces.is_empty() {
        return Err(ScanError::NoWirelessInterface);
    }

    let mut scan_started = false;
    for interface in interfaces {
        // SAFETY: interface GUID'i WlanEnumInterfaces çıktısından geliyor; SSID/IE/reserved parametreleri null verilerek genel tarama isteniyor.
        let scan_result =
            unsafe { WlanScan(handle.0, &interface.InterfaceGuid, null(), null(), null()) };

        if scan_result == ERROR_SUCCESS {
            scan_started = true;
        }
    }

    if !scan_started {
        return Err(ScanError::CommandFailed(
            "WlanScan hiçbir kablosuz arayüzde başlatılamadı".to_string(),
        ));
    }

    // WlanScan asenkron çalışır; netsh'in güncel tarama sonucunu okuyabilmesi için kısa süre bekliyoruz.
    thread::sleep(WINDOWS_SCAN_SETTLE_TIME);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn trigger_windows_wifi_scan() -> Result<(), ScanError> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn trigger_linux_wifi_scan() {
    // nmcli'nin cache'lenmiş eski sonuç döndürmesini azaltmak için yeniden tarama istiyoruz.
    // Hata olursa programı durdurmuyoruz; bazı dağıtımlarda bu çağrı izin/polkit isteyebilir.
    // Böyle durumlarda aşağıdaki `nmcli dev wifi list` yine mevcut cache'i okuyabilir.
    let _ = Command::new("nmcli")
        .args(["dev", "wifi", "rescan"])
        .status();
}

#[derive(Debug, PartialEq, Eq)]
struct ReportRow {
    ssid: String,
    is_open: bool,
    authentication: String,
    encryption: String,
    best_signal: Option<u8>,
    ap_count: usize,
    networks: Vec<WifiNetwork>,
}

#[must_use]
fn report_rows(networks: &[WifiNetwork]) -> Vec<ReportRow> {
    let mut grouped: BTreeMap<String, Vec<WifiNetwork>> = BTreeMap::new();

    for network in networks {
        grouped
            .entry(network.ssid.clone())
            .or_default()
            .push(network.clone());
    }

    grouped
        .into_iter()
        .map(|(ssid, mut networks)| {
            // Aynı SSID altındaki erişim noktalarını en güçlü sinyal önce olacak şekilde sıralıyoruz.
            // Bu, kullanıcıya bağlanmaya en yakın/iyi görünen AP'yi üstte gösterir.
            networks.sort_by(|left, right| {
                right
                    .details
                    .signal
                    .cmp(&left.details.signal)
                    .then_with(|| left.bssid.cmp(&right.bssid))
            });

            let is_open = networks.iter().any(|network| network.is_open);
            let authentication = join_unique(
                networks
                    .iter()
                    .map(|network| network.authentication.as_str())
                    .filter(|value| !value.trim().is_empty()),
            );
            let encryption = join_unique(
                networks
                    .iter()
                    .filter_map(|network| network.details.encryption.as_deref()),
            );
            let best_signal = networks
                .iter()
                .filter_map(|network| network.details.signal)
                .max();
            let ap_count = networks
                .iter()
                .filter_map(|network| network.bssid.as_deref())
                .collect::<BTreeSet<_>>()
                .len()
                .max(networks.len());

            ReportRow {
                ssid,
                is_open,
                authentication: empty_as_dash(authentication),
                encryption: empty_as_dash(encryption),
                best_signal,
                ap_count,
                networks,
            }
        })
        .collect()
}

fn join_unique<'a>(values: impl Iterator<Item = &'a str>) -> String {
    values
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ")
}

fn empty_as_dash(value: String) -> String {
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        value
    }
}

fn optional_str(value: Option<&str>) -> &str {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("-")
}

fn optional_u32(value: Option<u32>) -> String {
    value.map_or_else(|| "-".to_string(), |value| value.to_string())
}

fn signal_text(signal: Option<u8>) -> String {
    signal.map_or_else(|| "-".to_string(), |value| format!("{value}%"))
}

fn colorize(value: impl fmt::Display, color: &str) -> String {
    format!("{color}{value}{RESET}")
}

fn colored_signal(signal: Option<u8>) -> String {
    let text = signal_text(signal);
    match signal {
        Some(value) if value >= 75 => colorize(text, GREEN),
        Some(value) if value >= 45 => colorize(text, YELLOW),
        Some(_) => colorize(text, RED),
        None => colorize(text, DIM),
    }
}

fn colored_security(is_open: bool, authentication: &str, encryption: &str) -> String {
    if is_open {
        colorize("Şifresiz / Açık", RED)
    } else {
        colorize(format!("{authentication} / {encryption}"), GREEN)
    }
}

fn scan_wifi() -> Result<Vec<WifiNetwork>, ScanError> {
    let (raw, networks) = if cfg!(target_os = "windows") {
        trigger_windows_wifi_scan()?;
        let out = command_stdout("netsh", &["wlan", "show", "networks", "mode=Bssid"])?;
        if contains_any_lower(
            &out,
            &[
                "there is no wireless interface on the system",
                "sistemde kablosuz arabirim yok",
                "sistemde kablosuz ağ arabirimi yok",
            ],
        ) {
            return Err(ScanError::NoWirelessInterface);
        }
        let nets = parse_windows_netsh(&out);
        (out, nets)
    } else if cfg!(target_os = "linux") {
        let out = command_stdout(
            "nmcli",
            &[
                "--escape",
                "yes",
                "-t",
                "-f",
                "SSID,BSSID,SECURITY,SIGNAL,FREQ,CHAN,RATE",
                "dev",
                "wifi",
                "list",
            ],
        )?;
        if contains_any_lower(&out, &["no wifi device found", "no wireless"]) {
            return Err(ScanError::NoWirelessInterface);
        }
        let nets = parse_linux_nmcli(&out);
        (out, nets)
    } else if cfg!(target_os = "macos") {
        let airport = "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";
        let out = command_stdout(airport, &["-s"])?;
        let nets = parse_macos_airport(&out);
        (out, nets)
    } else {
        return Err(ScanError::UnsupportedPlatform(
            std::env::consts::OS.to_string(),
        ));
    };

    ensure_networks(&raw, networks)
}

fn print_report(networks: &[WifiNetwork]) {
    let rows = report_rows(networks);

    println!("{BOLD}{CYAN}Wi-Fi ağları{RESET}");
    println!(
        "{DIM}Toplam {} ağ adı, {} erişim noktası/BSSID bulundu.{RESET}\n",
        rows.len(),
        networks.len()
    );

    for (idx, row) in rows.iter().enumerate() {
        println!(
            "{BOLD}{BLUE}{:>2}. {:<32}{RESET} {}  {}  {}",
            idx + 1,
            row.ssid,
            colored_signal(row.best_signal),
            colored_security(row.is_open, &row.authentication, &row.encryption),
            colorize(format!("AP: {}", row.ap_count), CYAN)
        );

        println!(
            "   {DIM}{:<17} {:<28} {:>7} {:<8} {:<6} {:<10} {:>7} {:<12}Hızlar{RESET}",
            "BSSID", "Cihaz kimliği", "Sinyal", "Band", "Kanal", "Wi-Fi", "İstemci", "Kanal yükü"
        );

        for network in &row.networks {
            let details = &network.details;
            // Hızlar platforma göre farklı ayrıntıda gelebilir:
            // Windows basic/other rates verir, Linux çoğunlukla tek RATE değeri verir.
            let rates = match (&details.basic_rates, &details.other_rates) {
                (Some(basic), Some(other)) => format!("B:{basic} / O:{other}"),
                (Some(basic), None) => format!("B:{basic}"),
                (None, Some(other)) => format!("O:{other}"),
                (None, None) => "-".to_string(),
            };

            println!(
                "   {:<17} {:<28} {:>16} {:<8} {:<6} {:<10} {:>7} {:<12} {}",
                optional_str(network.bssid.as_deref()),
                device_identity(network.bssid.as_deref()),
                colored_signal(details.signal),
                optional_str(details.band.as_deref()),
                optional_str(details.channel.as_deref()),
                optional_str(details.radio_type.as_deref()),
                optional_u32(details.connected_stations),
                optional_str(details.channel_utilization.as_deref()),
                rates
            );
        }

        println!();
    }
}

fn main() {
    match scan_wifi() {
        Ok(networks) => print_report(&networks),
        Err(err) => {
            eprintln!("Hata: {err}");
            if let Some(hint) = err.user_hint() {
                eprintln!("Öneri: {hint}");
            }
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_windows_detects_open_and_secured() {
        let sample = r#"
Interface name : Wi-Fi
There are 2 networks currently visible.

SSID 1 : CafeGuest
    Network type            : Infrastructure
    Authentication          : Open
    Encryption              : None

SSID 2 : HomeSecure
    Network type            : Infrastructure
    Authentication          : WPA2-Personal
    Encryption              : CCMP
"#;

        let parsed = parse_windows_netsh(sample);
        assert_eq!(parsed.len(), 2);

        let guest = parsed
            .iter()
            .find(|network| network.ssid == "CafeGuest")
            .expect("CafeGuest bulunmalı");
        assert!(guest.is_open);

        let home = parsed
            .iter()
            .find(|network| network.ssid == "HomeSecure")
            .expect("HomeSecure bulunmalı");
        assert!(!home.is_open);
    }

    #[test]
    fn parse_windows_supports_turkish_authentication_label() {
        let sample = r#"
SSID 1 : Misafir
    Kimlik Doğrulaması      : Açık
"#;

        let parsed = parse_windows_netsh(sample);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].ssid, "Misafir");
        assert!(parsed[0].is_open);
    }

    #[test]
    fn parse_windows_preserves_multiple_bssids_for_same_ssid() {
        let sample = r#"
SSID 1 : OfficeNet
    Authentication          : WPA2-Personal
    BSSID 1                 : aa:bb:cc:dd:ee:ff
         Signal             : 89%
         Radio type         : 802.11ax
    BSSID 2                 : 11:22:33:44:55:66
         Signal             : 72%
         Radio type         : 802.11ac

SSID 2 : OfficeNet
    Authentication          : WPA2-Personal
    BSSID 1                 : 77:88:99:aa:bb:cc
"#;

        let parsed = parse_windows_netsh(sample);
        assert_eq!(parsed.len(), 3);
        assert!(parsed.iter().all(|network| network.ssid == "OfficeNet"));
        assert!(
            parsed
                .iter()
                .any(|network| network.bssid.as_deref() == Some("aa:bb:cc:dd:ee:ff"))
        );
        assert!(
            parsed
                .iter()
                .any(|network| network.bssid.as_deref() == Some("11:22:33:44:55:66"))
        );
        assert!(
            parsed
                .iter()
                .any(|network| network.bssid.as_deref() == Some("77:88:99:aa:bb:cc"))
        );
    }

    #[test]
    fn report_rows_collapses_same_ssid_to_single_output_row() {
        let networks = vec![
            WifiNetwork::new("OfficeNet", Some("aa:bb:cc:dd:ee:ff"), "WPA2-Personal"),
            WifiNetwork::new("OfficeNet", Some("11:22:33:44:55:66"), "WPA2-Personal"),
            WifiNetwork::new("CafeGuest", None::<String>, "Open"),
        ];

        let rows = report_rows(&networks);

        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.ssid == "CafeGuest"));
        let office = rows
            .iter()
            .find(|row| row.ssid == "OfficeNet")
            .expect("OfficeNet tek satır olmalı");
        assert_eq!(office.ap_count, 2);
        assert_eq!(office.networks.len(), 2);
        assert!(!office.is_open);
    }

    #[test]
    fn parse_windows_reads_detailed_bssid_fields() {
        let sample = r#"
SSID 1 : OfficeNet
    Authentication          : WPA2-Personal
    Encryption              : CCMP
    BSSID 1                 : aa:bb:cc:dd:ee:ff
         Signal             : 89%
         Radio type         : 802.11ax
         Band               : 5 GHz
         Channel            : 100
         Bss Load:
             Connected Stations:         2
             Channel Utilization:        34 (13 %)
         Basic rates (Mbps) : 6 12 24
         Other rates (Mbps) : 9 18 36 48 54
"#;

        let parsed = parse_windows_netsh(sample);
        assert_eq!(parsed.len(), 1);

        let details = &parsed[0].details;
        assert_eq!(details.encryption.as_deref(), Some("CCMP"));
        assert_eq!(details.signal, Some(89));
        assert_eq!(details.radio_type.as_deref(), Some("802.11ax"));
        assert_eq!(details.band.as_deref(), Some("5 GHz"));
        assert_eq!(details.channel.as_deref(), Some("100"));
        assert_eq!(details.connected_stations, Some(2));
        assert_eq!(details.channel_utilization.as_deref(), Some("34 (13 %)"));
        assert_eq!(details.basic_rates.as_deref(), Some("6 12 24"));
        assert_eq!(details.other_rates.as_deref(), Some("9 18 36 48 54"));
    }

    #[test]
    fn parse_linux_nmcli_detects_open_and_escaped_ssid() {
        let sample = "Cafe\\:Guest:aa\\:bb\\:cc\\:dd\\:ee\\:ff:--:82:2412:6:54 Mbit/s\nHomeSecure:11\\:22\\:33\\:44\\:55\\:66:WPA2:71:5180:36:866 Mbit/s\n";
        let parsed = parse_linux_nmcli(sample);

        assert_eq!(parsed.len(), 2);
        assert!(
            parsed
                .iter()
                .any(|network| network.ssid == "Cafe:Guest" && network.is_open)
        );
        assert!(
            parsed
                .iter()
                .any(|network| network.ssid == "HomeSecure" && !network.is_open)
        );

        let guest = parsed
            .iter()
            .find(|network| network.ssid == "Cafe:Guest")
            .expect("Cafe:Guest bulunmalı");
        assert_eq!(guest.details.signal, Some(82));
        assert_eq!(guest.details.band.as_deref(), Some("2.4 GHz"));
        assert_eq!(guest.details.channel.as_deref(), Some("6"));
        assert_eq!(guest.details.radio_type.as_deref(), Some("802.11a/g"));

        let home = parsed
            .iter()
            .find(|network| network.ssid == "HomeSecure")
            .expect("HomeSecure bulunmalı");
        assert_eq!(home.details.signal, Some(71));
        assert_eq!(home.details.band.as_deref(), Some("5 GHz"));
        assert_eq!(home.details.channel.as_deref(), Some("36"));
        assert_eq!(home.details.radio_type.as_deref(), Some("802.11ac/ax"));
    }

    #[test]
    fn bssid_vendor_identity_is_reported_when_known() {
        assert_eq!(
            normalize_bssid_for_oui("54:78:f0:a5:90:c1").as_deref(),
            Some("5478F0")
        );
        assert_eq!(vendor_from_bssid("54:78:f0:a5:90:c1"), "ZTE / ISP CPE");
        assert_eq!(vendor_from_bssid("ff:ff:ff:ff:ff:ff"), "Bilinmiyor");
        assert_eq!(device_identity(None), "-");
    }

    #[test]
    fn parse_macos_airport_supports_ssids_with_spaces() {
        let sample = r#"
                            SSID BSSID             RSSI CHANNEL HT CC SECURITY
                       Cafe Guest aa:bb:cc:dd:ee:ff -48  6       Y  TR NONE
                       Home Secure 11:22:33:44:55:66 -61  11      Y  TR WPA2(PSK/AES/AES)
"#;

        let parsed = parse_macos_airport(sample);
        assert_eq!(parsed.len(), 2);
        assert!(
            parsed
                .iter()
                .any(|network| network.ssid == "Cafe Guest" && network.is_open)
        );
        assert!(
            parsed
                .iter()
                .any(|network| network.ssid == "Home Secure" && !network.is_open)
        );
    }

    #[test]
    fn open_auth_helper_avoids_enhanced_open_false_positive() {
        assert!(is_open_auth("Open"));
        assert!(is_open_auth("Açık"));
        assert!(is_open_auth("--"));
        assert!(!is_open_auth("Enhanced Open"));
        assert!(!is_open_auth("WPA2-Personal"));
    }

    #[test]
    fn ensure_networks_reports_empty_scan_separately() {
        let result = ensure_networks("", Vec::new());
        assert!(matches!(result, Err(ScanError::NoNetworksVisible)));
    }

    #[test]
    fn no_wireless_interface_detected() {
        let sample = "There is no wireless interface on the system.";
        assert!(contains_any_lower(
            sample,
            &["there is no wireless interface on the system"]
        ));
    }
}
