//! Raw RFC 5322 → `MailMessage` berisi teks body ternormalisasi.
use mail_parser::{MessageParser, PartKind};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

/// Batas panjang `raw_snippet` yang disimpan (karakter, bukan byte).
pub const SNIPPET_CHARS: usize = 4000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailMessage {
    /// Header `Message-ID` tanpa `<>`, atau `sha256:<hex>` dari raw bila tidak ada.
    pub message_id: String,
    /// Alamat pengirim saja, lowercase.
    pub from: String,
    pub subject: String,
    pub sent_at: Option<OffsetDateTime>,
    /// Body teks; HTML dikonversi ke teks, whitespace dirapikan.
    pub text: String,
}

/// ID pengganti `Message-ID`: `sha256:<hex>` dari raw.
pub fn raw_message_id(raw: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(raw))
}

/// `None` bila raw bukan email yang bisa dibaca.
pub fn parse_message(raw: &[u8]) -> Option<MailMessage> {
    let msg = MessageParser::new().parse(raw)?;
    let message_id = match msg.message_id().map(str::trim).filter(|s| !s.is_empty()) {
        Some(id) => id.to_owned(),
        None => raw_message_id(raw),
    };
    let from = msg
        .from()
        .and_then(|list| list.first())
        .and_then(|m| m.address())
        .unwrap_or_default()
        .to_lowercase();
    let subject = msg.subject().map(collapse_line).unwrap_or_default();
    let sent_at = msg
        .date()
        .and_then(|d| OffsetDateTime::from_unix_timestamp(d.to_timestamp()).ok());
    let body = msg.text_body().next().and_then(|part| match part.kind() {
        PartKind::Text => part.text().map(|t| t.into_owned()),
        PartKind::Html => part.text().and_then(|html| html_to_text(&html)),
        _ => None,
    });
    Some(MailMessage {
        message_id,
        from,
        subject,
        sent_at,
        text: normalize(&body.unwrap_or_default()),
    })
}

/// HTML → teks tanpa border tabel & footnote link; sel satu baris tabel tetap di satu baris.
fn html_to_text(html: &str) -> Option<String> {
    html2text::config::plain_no_decorate()
        .link_footnotes(false)
        .no_table_borders()
        .string_from_read(html.as_bytes(), HTML_WIDTH)
        .ok()
}

/// Lebar render HTML; cukup lebar agar baris label/nilai tidak terpotong.
const HTML_WIDTH: usize = 200;

/// Spasi berurutan (termasuk NBSP & tab) → satu spasi, tiap baris di-trim,
/// baris kosong beruntun → satu baris kosong.
fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank = false;
    for line in text.lines().map(collapse_line) {
        if line.is_empty() {
            blank = !out.is_empty();
            continue;
        }
        if !out.is_empty() {
            out.push_str(if blank { "\n\n" } else { "\n" });
        }
        blank = false;
        out.push_str(&line);
    }
    out
}

fn collapse_line(line: &str) -> String {
    line.split(|c: char| c.is_whitespace())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Potong ke [`SNIPPET_CHARS`] karakter.
pub fn snippet(text: &str) -> String {
    text.chars().take(SNIPPET_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> MailMessage {
        parse_message(raw.as_bytes()).expect("email terbaca")
    }

    #[test]
    fn plain_text_body() {
        let m = parse(
            "Message-ID: <a@b>\r\nFrom: \"Bank\" <Notif@Bank.CO.ID>\r\nSubject: Halo\r\n\
             Date: Mon, 26 Oct 2026 20:30:00 +0000\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n\
             Jumlah Rp 21.000\r\n",
        );
        assert_eq!(m.message_id, "a@b");
        assert_eq!(m.from, "notif@bank.co.id");
        assert_eq!(m.subject, "Halo");
        assert!(m.sent_at.is_some());
        assert!(m.text.contains("Jumlah Rp 21.000"), "{}", m.text);
    }

    #[test]
    fn html_only_body_is_converted() {
        let raw = b"Message-ID: <h@b>\r\nFrom: a@b.c\r\nSubject: x\r\n\
            Content-Type: text/html; charset=iso-8859-1\r\n\r\n\
            <html><body><table><tr><td>Nominal</td><td>Rp&nbsp;150.000,00</td></tr></table>\
            <p><a href=\"https://x.test/track\">Caf\xe9</a></p></body></html>\r\n";
        let m = parse_message(raw).expect("email terbaca");
        assert!(m.text.contains("Nominal Rp 150.000,00"), "{}", m.text);
        assert!(!m.text.contains('│') && !m.text.contains('─'), "{}", m.text);
        assert!(m.text.contains("Café"), "{}", m.text);
        assert!(
            !m.text.contains('<') && !m.text.contains("x.test"),
            "{}",
            m.text
        );
    }

    #[test]
    fn missing_message_id_uses_hash() {
        let raw = "From: a@b.c\r\nSubject: x\r\n\r\nhalo\r\n";
        let a = parse(raw);
        assert!(a.message_id.starts_with("sha256:"), "{}", a.message_id);
        assert_eq!(a.message_id.len(), "sha256:".len() + 64);
        assert_eq!(parse(raw).message_id, a.message_id);
    }

    #[test]
    fn whitespace_is_collapsed() {
        let m = parse(
            "Message-ID: <w@b>\r\nFrom: a@b.c\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n\
             Total\t:\u{a0}\u{a0} IDR  41,700.00   \r\n\r\n\r\n\r\nBaris  dua\r\n",
        );
        assert_eq!(m.text, "Total : IDR 41,700.00\n\nBaris dua");
        assert_eq!(m.subject, "");
    }

    #[test]
    fn snippet_truncates_on_char_boundary() {
        let s = "é".repeat(SNIPPET_CHARS + 1);
        assert_eq!(snippet(&s).chars().count(), SNIPPET_CHARS);
        assert_eq!(snippet("pendek"), "pendek");
    }
}
