use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::bookgrep::error::{BookgrepError, Result};

const TESSERACT: &str = "tesseract";
const PDFTOPPM: &str = "pdftoppm";

pub fn extract_pdf_text(path: &Path) -> Result<String> {
    ensure_ocr_tools_available()?;

    let temp_dir = create_temp_dir()?;
    let prefix = temp_dir.join("page");
    let convert_result = convert_pdf_to_png(path, &prefix)
        .and_then(|()| collect_page_images(&temp_dir))
        .and_then(|images| ocr_images(&images));
    let _ = fs::remove_dir_all(&temp_dir);
    convert_result
}

fn ensure_ocr_tools_available() -> Result<()> {
    let mut missing = Vec::new();
    if !command_available(TESSERACT) {
        missing.push(TESSERACT);
    }
    if !command_available(PDFTOPPM) {
        missing.push(PDFTOPPM);
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(BookgrepError::Ocr(format!(
            "missing command(s): {}. Install Tesseract OCR and Poppler, then ensure `tesseract` and `pdftoppm` are on PATH.",
            missing.join(", ")
        )))
    }
}

fn command_available(command: &str) -> bool {
    ["--version", "-v"].iter().any(|arg| {
        Command::new(command)
            .arg(arg)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
}

fn create_temp_dir() -> Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let dir = env::temp_dir().join(format!("bookgrep-ocr-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|err| BookgrepError::Ocr(err.to_string()))?;
    Ok(dir)
}

fn convert_pdf_to_png(path: &Path, prefix: &Path) -> Result<()> {
    let output = Command::new(PDFTOPPM)
        .args(["-png", "-r", "300"])
        .arg(path)
        .arg(prefix)
        .stdin(Stdio::null())
        .output()
        .map_err(|err| BookgrepError::Ocr(format!("could not run pdftoppm: {err}")))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(BookgrepError::Ocr(format!(
            "pdftoppm failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn collect_page_images(temp_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut images = fs::read_dir(temp_dir)
        .map_err(|err| BookgrepError::Ocr(err.to_string()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        })
        .collect::<Vec<_>>();
    images.sort();

    if images.is_empty() {
        Err(BookgrepError::Ocr(
            "pdftoppm did not produce page images".into(),
        ))
    } else {
        Ok(images)
    }
}

fn ocr_images(images: &[PathBuf]) -> Result<String> {
    let mut pages = Vec::new();
    for (index, image) in images.iter().enumerate() {
        let output = Command::new(TESSERACT)
            .arg(image)
            .arg("stdout")
            .stdin(Stdio::null())
            .output()
            .map_err(|err| BookgrepError::Ocr(format!("could not run tesseract: {err}")))?;

        if !output.status.success() {
            return Err(BookgrepError::Ocr(format!(
                "tesseract failed on page {}: {}",
                index + 1,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !text.is_empty() {
            pages.push(format!("Page {}\n{text}", index + 1));
        }
    }

    Ok(pages.join("\x0C"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_command() {
        assert!(!command_available("bookgrep-command-that-should-not-exist"));
    }
}
