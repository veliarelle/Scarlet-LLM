use crate::types::Attachment;
use base64::Engine as _;
use flate2::read::DeflateDecoder;
use serde::Deserialize;
use std::io::Read;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RawAttachmentInput {
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

#[tauri::command]
pub fn prepare_attachments(files: Vec<RawAttachmentInput>) -> Result<Vec<Attachment>, String> {
    files
        .into_iter()
        .map(|file| {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&file.data)
                .map_err(|e| format!("decode {}: {e}", file.name))?;
            let mime_type = if file.mime_type.trim().is_empty()
                || file.mime_type == "application/octet-stream"
            {
                mime_from_name(&file.name).to_string()
            } else {
                file.mime_type
            };
            let text = extract_text(&file.name, &mime_type, &bytes);
            Ok(Attachment {
                id: Uuid::new_v4().to_string(),
                name: file.name,
                mime_type,
                data: file.data,
                text,
            })
        })
        .collect()
}

#[tauri::command]
pub fn read_dropped_files(paths: Vec<String>) -> Result<Vec<Attachment>, String> {
    let mut out = Vec::new();
    for raw_path in paths {
        let path = Path::new(&raw_path);
        if !path.is_file() {
            continue;
        }
        let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("attachment")
            .to_string();
        let mime_type = mime_from_name(&name).to_string();
        let text = extract_text(&name, &mime_type, &bytes);
        out.push(Attachment {
            id: Uuid::new_v4().to_string(),
            name,
            mime_type,
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
            text,
        });
    }
    Ok(out)
}

fn mime_from_name(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "log" | "ini" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "csv" => "text/csv",
        "json" | "jsonl" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "yaml" | "yml" => "application/yaml",
        "js" | "ts" | "css" | "py" | "rs" | "go" | "java" | "c" | "cpp" | "cs" | "sh" | "sql" => {
            "text/plain"
        }
        "rtf" => "application/rtf",
        "odt" => "application/vnd.oasis.opendocument.text",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

pub(crate) fn extract_text_from_base64(name: &str, mime: &str, data: &str) -> Option<String> {
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()
        .and_then(|bytes| extract_text(name, mime, &bytes))
}

fn extract_text(name: &str, mime: &str, bytes: &[u8]) -> Option<String> {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if mime.starts_with("text/")
        || matches!(
            mime,
            "application/json" | "application/xml" | "application/yaml" | "application/javascript"
        )
    {
        return String::from_utf8(bytes.to_vec())
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    if ext == "rtf" || mime == "application/rtf" {
        return String::from_utf8(bytes.to_vec())
            .ok()
            .map(|s| strip_rtf(&s))
            .filter(|s| !s.is_empty());
    }
    if ext == "odt" || mime == "application/vnd.oasis.opendocument.text" {
        return extract_zip_entry(bytes, "content.xml")
            .map(|xml| xml_to_text(&xml))
            .filter(|s| !s.is_empty());
    }
    if ext == "docx"
        || mime == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    {
        return extract_zip_entry(bytes, "word/document.xml")
            .map(|xml| xml_to_text(&xml))
            .filter(|s| !s.is_empty());
    }
    None
}

fn extract_zip_entry(bytes: &[u8], wanted: &str) -> Option<String> {
    if let Some(text) = extract_zip_entry_from_central_directory(bytes, wanted) {
        return Some(text);
    }

    let mut offset = 0usize;
    while offset + 30 <= bytes.len() {
        if bytes.get(offset..offset + 4)? != [0x50, 0x4b, 0x03, 0x04] {
            offset += 1;
            continue;
        }
        let method = u16::from_le_bytes([bytes[offset + 8], bytes[offset + 9]]);
        let compressed_size = u32::from_le_bytes([
            bytes[offset + 18],
            bytes[offset + 19],
            bytes[offset + 20],
            bytes[offset + 21],
        ]) as usize;
        let name_len = u16::from_le_bytes([bytes[offset + 26], bytes[offset + 27]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[offset + 28], bytes[offset + 29]]) as usize;
        let name_start = offset + 30;
        let data_start = name_start.checked_add(name_len)?.checked_add(extra_len)?;
        let data_end = data_start.checked_add(compressed_size)?;
        if data_end > bytes.len() {
            return None;
        }
        let name = std::str::from_utf8(bytes.get(name_start..name_start + name_len)?).ok()?;
        if name == wanted {
            let payload = bytes.get(data_start..data_end)?;
            return match method {
                0 => String::from_utf8(payload.to_vec()).ok(),
                8 => {
                    let mut decoder = DeflateDecoder::new(payload);
                    let mut out = String::new();
                    decoder.read_to_string(&mut out).ok()?;
                    Some(out)
                }
                _ => None,
            };
        }
        offset = data_end.max(offset + 1);
    }
    None
}

fn extract_zip_entry_from_central_directory(bytes: &[u8], wanted: &str) -> Option<String> {
    let mut offset = 0usize;
    while offset + 46 <= bytes.len() {
        if bytes.get(offset..offset + 4)? != [0x50, 0x4b, 0x01, 0x02] {
            offset += 1;
            continue;
        }

        let method = le_u16(bytes, offset + 10)?;
        let compressed_size = le_u32(bytes, offset + 20)? as usize;
        let name_len = le_u16(bytes, offset + 28)? as usize;
        let extra_len = le_u16(bytes, offset + 30)? as usize;
        let comment_len = le_u16(bytes, offset + 32)? as usize;
        let local_header_offset = le_u32(bytes, offset + 42)? as usize;
        let name_start = offset + 46;
        let name_end = name_start.checked_add(name_len)?;
        let entry_end = name_end.checked_add(extra_len)?.checked_add(comment_len)?;
        if entry_end > bytes.len() {
            return None;
        }

        let name = std::str::from_utf8(bytes.get(name_start..name_end)?).ok()?;
        if name == wanted {
            let data_start = zip_local_data_start(bytes, local_header_offset)?;
            let data_end = data_start.checked_add(compressed_size)?;
            let payload = bytes.get(data_start..data_end)?;
            return inflate_zip_payload(method, payload);
        }

        offset = entry_end.max(offset + 1);
    }
    None
}

fn zip_local_data_start(bytes: &[u8], offset: usize) -> Option<usize> {
    if bytes.get(offset..offset + 4)? != [0x50, 0x4b, 0x03, 0x04] {
        return None;
    }
    let name_len = le_u16(bytes, offset + 26)? as usize;
    let extra_len = le_u16(bytes, offset + 28)? as usize;
    offset
        .checked_add(30)?
        .checked_add(name_len)?
        .checked_add(extra_len)
}

fn inflate_zip_payload(method: u16, payload: &[u8]) -> Option<String> {
    match method {
        0 => String::from_utf8(payload.to_vec()).ok(),
        8 => {
            let mut decoder = DeflateDecoder::new(payload);
            let mut out = String::new();
            decoder.read_to_string(&mut out).ok()?;
            Some(out)
        }
        _ => None,
    }
}

fn le_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
    ]))
}

fn le_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
        *bytes.get(offset + 2)?,
        *bytes.get(offset + 3)?,
    ]))
}

fn xml_to_text(xml: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut tag = String::new();
    for c in xml.chars() {
        match c {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' => {
                in_tag = false;
                push_space_for_tag(&mut out, &tag);
            }
            _ if in_tag => tag.push(c),
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode_xml_entities(&out)
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn push_space_for_tag(out: &mut String, tag: &str) {
    let normalized = tag
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("");
    match normalized {
        "text:p" | "text:h" | "w:p" | "w:br" | "br" | "p" | "h" => out.push('\n'),
        "text:tab" | "w:tab" | "tab" => out.push('\t'),
        "text:s" => out.push(' '),
        _ => out.push(' '),
    }
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn strip_rtf(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' | '}' => out.push(' '),
            '\\' => {
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    chars.next();
                    chars.next();
                } else {
                    while matches!(chars.peek(), Some(ch) if ch.is_ascii_alphabetic() || ch.is_ascii_digit() || *ch == '-')
                    {
                        chars.next();
                    }
                    if chars.peek() == Some(&' ') {
                        chars.next();
                    }
                    out.push(' ');
                }
            }
            _ => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
