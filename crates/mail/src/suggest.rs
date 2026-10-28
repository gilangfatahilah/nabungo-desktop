//! Saran rule dari email yang gagal dibaca (EML-04): titik awal yang lolos uji pada email
//! contoh, untuk dirapikan user di rule editor.
use std::sync::LazyLock;

use regex::{Regex, escape};

use crate::message::MailMessage;
use crate::rule::{Rule, RuleKind, normalize_amount, parse_date};

/// Panjang maksimum nama rule hasil saran (karakter).
const NAME_CHARS: usize = 60;
/// Label (teks sebelum nominal/tanggal di satu baris) yang lebih panjang dari ini, atau yang
/// berisi angka, dianggap memuat data yang berubah tiap email dan tidak dipakai sebagai pola.
const LABEL_CHARS: usize = 40;
/// Prioritas rule hasil saran: di atas rule bawaan (0).
const PRIORITY: i64 = 10;
const GENERIC_AMOUNT: &str = r"(?:Rp\.?|IDR) ?([\d.,]+)";

static AMOUNT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(Rp\.?|IDR) ?(\d[\d.,]*)").expect("regex valid"));

/// (pola teks tanggal, pola untuk rule, format).
static DATES: LazyLock<[(Regex, &str, &str); 3]> = LazyLock::new(|| {
    [
        (
            Regex::new(r"\d{1,2} [A-Za-z]+ \d{4}").expect("regex valid"),
            r"(\d{1,2} \w+ \d{4})",
            "dd mmm yyyy",
        ),
        (
            Regex::new(r"\d{1,2}/\d{1,2}/\d{4}").expect("regex valid"),
            r"(\d{1,2}/\d{1,2}/\d{4})",
            "dd/mm/yyyy",
        ),
        (
            Regex::new(r"\d{1,2}-\d{1,2}-\d{4}").expect("regex valid"),
            r"(\d{1,2}-\d{1,2}-\d{4})",
            "dd-mm-yyyy",
        ),
    ]
});

/// Label yang stabil antaremail: pendek dan tanpa angka.
fn stable_label(prefix: &str) -> bool {
    prefix.chars().count() <= LABEL_CHARS && !prefix.chars().any(|c| c.is_ascii_digit())
}

pub fn suggest(msg: &MailMessage) -> Rule {
    let domain = msg
        .from
        .rsplit_once('@')
        .map_or(msg.from.as_str(), |(_, d)| d)
        .trim()
        .to_owned();
    let subject = msg.subject.trim();
    let name = if subject.is_empty() {
        domain.clone()
    } else {
        subject
            .chars()
            .take(NAME_CHARS)
            .collect::<String>()
            .trim()
            .to_owned()
    };
    let (date_regex, date_format) = suggest_date(&msg.text).unzip();
    Rule {
        id: None,
        name,
        sender_pattern: domain,
        subject_regex: (!subject.is_empty()).then(|| format!("^{}$", escape(subject))),
        amount_regex: suggest_amount(&msg.text).unwrap_or_else(|| GENERIC_AMOUNT.into()),
        payee_regex: None,
        date_regex,
        date_format,
        kind: RuleKind::Expense,
        account_id: None,
        counter_account_id: None,
        default_category_id: None,
        priority: PRIORITY,
    }
}

/// Baris pertama berisi nominal yang terbaca; dianker ke labelnya bila label stabil.
fn suggest_amount(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let m = AMOUNT
            .captures_iter(line)
            .find(|c| normalize_amount(&c[2]).is_ok_and(|a| a > 0))?;
        let whole = m.get(0)?;
        let prefix = &line[..whole.start()];
        if prefix.trim().is_empty() || !stable_label(prefix) {
            return None;
        }
        let end = if line[whole.end()..].trim().is_empty() {
            "$"
        } else {
            ""
        };
        Some(format!(
            "^{}{} ?([\\d.,]+){end}",
            escape(prefix),
            escape(&m[1])
        ))
    })
}

/// Baris berlabel stabil pertama yang memuat tanggal terbaca.
fn suggest_date(text: &str) -> Option<(String, String)> {
    text.lines().find_map(|line| {
        DATES.iter().find_map(|(re, pattern, format)| {
            let m = re.find(line)?;
            let prefix = &line[..m.start()];
            if prefix.trim().is_empty() || !stable_label(prefix) {
                return None;
            }
            parse_date(m.as_str(), format)?;
            Some((
                format!("^{}{pattern}", escape(prefix)),
                (*format).to_owned(),
            ))
        })
    })
}
