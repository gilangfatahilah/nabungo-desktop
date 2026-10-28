//! Rule engine: cocokkan teks email notifikasi ke nominal, payee, dan tanggal.
//!
//! Rule dipilih berurutan (pemanggil sudah mengurutkan `priority` DESC lalu `name`):
//! rule pertama yang `sender_pattern`-nya substring alamat From, `subject_regex`-nya cocok
//! (bila ada), dan punya ≥1 match `amount_regex` dipakai. Bila rule itu gagal membaca
//! nominal/tanggal, seluruh email menjadi [`Outcome::Unparsed`] — tidak jatuh ke rule lain.
use nabungo_core::amount::{AmountParseError, parse_amount};
use regex::{Regex, RegexBuilder};
use time::{Date, Month, UtcOffset};
use uuid::Uuid;

use crate::message::MailMessage;

/// Jenis transaksi yang dihasilkan rule. Transfer: `TransferOut` dari akun rule ke akun
/// lawan, `TransferIn` dari akun lawan ke akun rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    Income,
    Expense,
    TransferOut,
    TransferIn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// `None` untuk rule bawaan sebelum disimpan.
    pub id: Option<Uuid>,
    pub name: String,
    /// Substring alamat From, tanpa beda huruf besar/kecil.
    pub sender_pattern: String,
    pub subject_regex: Option<String>,
    /// Grup 1 = nominal; setiap match menjadi satu item.
    pub amount_regex: String,
    pub payee_regex: Option<String>,
    pub date_regex: Option<String>,
    /// `dd/mm/yyyy`, `dd-mm-yyyy`, atau `dd mmm yyyy`.
    pub date_format: Option<String>,
    pub kind: RuleKind,
    pub account_id: Option<Uuid>,
    pub counter_account_id: Option<Uuid>,
    pub default_category_id: Option<Uuid>,
    pub priority: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    /// Urutan match `amount_regex` di body (kunci idempotency bersama Message-ID).
    pub idx: u32,
    pub amount: i64,
    pub payee: Option<String>,
    pub date: Date,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `rule` = indeks ke slice rule yang diberikan.
    Parsed {
        rule: usize,
        items: Vec<Captured>,
    },
    Unparsed,
}

/// Field rule yang tidak valid (lihat [`validate`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleField {
    Name,
    Sender,
    Subject,
    Amount,
    Payee,
    Date,
    DateFormat,
}

/// Format yang dikenali [`parse_date`].
pub const DATE_FORMATS: [&str; 3] = ["dd/mm/yyyy", "dd-mm-yyyy", "dd mmm yyyy"];

/// Langkah pertama yang gagal saat satu rule dicocokkan ke email. `Sender`, `Subject`, dan
/// `NoAmount` berarti rule dilewati (rule berikutnya dicoba); sisanya membuat email unparsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceFail {
    Sender,
    Subject,
    NoAmount,
    /// Teks grup nominal yang tidak terbaca (atau nol).
    AmountInvalid(String),
    /// Tanggal tidak ada di body dan email tanpa header `Date`.
    NoDate,
    /// Teks grup tanggal tidak terbaca dan email tanpa header `Date`.
    DateInvalid(String),
}

impl TraceFail {
    /// Rule dilewati, bukan email gagal dibaca.
    fn skips(&self) -> bool {
        matches!(self, Self::Sender | Self::Subject | Self::NoAmount)
    }
}

/// Hasil langkah demi langkah satu rule pada satu email (pratinjau rule editor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleTrace {
    pub sender_ok: bool,
    /// `None` = rule tanpa `subject_regex`.
    pub subject_ok: Option<bool>,
    /// Teks grup 1 `amount_regex` apa adanya.
    pub amounts: Vec<String>,
    pub payee: Option<String>,
    /// Teks grup 1 `date_regex`.
    pub date_text: Option<String>,
    /// Tanggal diambil dari header `Date` (body tidak punya tanggal yang terbaca).
    pub date_from_header: bool,
    pub result: Result<Vec<Captured>, TraceFail>,
}

struct Compiled {
    sender: String,
    subject: Option<Regex>,
    amount: Regex,
    payee: Option<Regex>,
    date: Option<(Regex, String)>,
}

/// Rule yang regex-nya sudah di-compile, untuk memproses banyak email.
pub struct CompiledRules {
    /// `None` = rule tidak valid (regex rusak / sender kosong), dilewati.
    compiled: Vec<Option<Compiled>>,
    source: Vec<Rule>,
}

fn regex(pattern: &str) -> Result<Regex, regex::Error> {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .multi_line(true)
        .build()
}

fn compile(rule: &Rule) -> Result<Compiled, regex::Error> {
    let opt = |p: &Option<String>| p.as_deref().map(regex).transpose();
    Ok(Compiled {
        sender: rule.sender_pattern.trim().to_lowercase(),
        subject: opt(&rule.subject_regex)?,
        amount: regex(&rule.amount_regex)?,
        payee: opt(&rule.payee_regex)?,
        date: match (&rule.date_regex, &rule.date_format) {
            (Some(re), Some(fmt)) => Some((regex(re)?, fmt.clone())),
            _ => None,
        },
    })
}

/// Rule bisa disimpan: nama & pengirim terisi, regex valid dan punya grup tangkap, pola &
/// format tanggal berpasangan. Akun/kategori diperiksa pemanggil.
pub fn validate(rule: &Rule) -> Result<(), RuleField> {
    let with_group = |p: &str, field| match regex(p) {
        Ok(re) if re.captures_len() >= 2 => Ok(()),
        _ => Err(field),
    };
    if rule.name.trim().is_empty() {
        return Err(RuleField::Name);
    }
    if rule.sender_pattern.trim().is_empty() {
        return Err(RuleField::Sender);
    }
    if let Some(p) = &rule.subject_regex {
        regex(p).map_err(|_| RuleField::Subject)?;
    }
    with_group(&rule.amount_regex, RuleField::Amount)?;
    if let Some(p) = &rule.payee_regex {
        with_group(p, RuleField::Payee)?;
    }
    match (&rule.date_regex, &rule.date_format) {
        (Some(p), Some(f)) => {
            with_group(p, RuleField::Date)?;
            if !DATE_FORMATS.contains(&f.as_str()) {
                return Err(RuleField::DateFormat);
            }
        }
        (Some(_), None) => return Err(RuleField::DateFormat),
        (None, Some(_)) => return Err(RuleField::Date),
        (None, None) => {}
    }
    Ok(())
}

/// Cocokkan satu rule ke email dan jelaskan langkah yang gagal. Satu jalur dengan
/// [`CompiledRules::apply`]: hasilnya sama dengan yang dicatat sync.
pub fn trace(
    rule: &Rule,
    msg: &MailMessage,
    local_offset: UtcOffset,
) -> Result<RuleTrace, RuleField> {
    validate(rule)?;
    let compiled = compile(rule).map_err(|_| RuleField::Amount)?;
    Ok(run(&compiled, &msg.from.to_lowercase(), msg, local_offset))
}

/// `from` = alamat pengirim lowercase (dihitung sekali per email oleh `apply`).
fn run(rule: &Compiled, from: &str, msg: &MailMessage, local_offset: UtcOffset) -> RuleTrace {
    let sender_ok = from.contains(&rule.sender);
    let subject_ok = rule.subject.as_ref().map(|re| re.is_match(&msg.subject));
    let mut t = RuleTrace {
        sender_ok,
        subject_ok,
        amounts: Vec::new(),
        payee: None,
        date_text: None,
        date_from_header: false,
        result: Err(TraceFail::Sender),
    };
    if !sender_ok {
        return t;
    }
    if subject_ok == Some(false) {
        t.result = Err(TraceFail::Subject);
        return t;
    }
    t.amounts = rule
        .amount
        .captures_iter(&msg.text)
        .filter_map(|c| c.get(1))
        .map(|m| m.as_str().to_owned())
        .collect();
    if t.amounts.is_empty() {
        t.result = Err(TraceFail::NoAmount);
        return t;
    }
    t.payee = rule
        .payee
        .as_ref()
        .and_then(|re| re.captures(&msg.text)?.get(1))
        .map(|m| m.as_str().trim().to_owned())
        .filter(|p| !p.is_empty());
    let mut body_date = None;
    if let Some((re, fmt)) = &rule.date {
        t.date_text = re
            .captures(&msg.text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_owned());
        body_date = t.date_text.as_deref().and_then(|d| parse_date(d, fmt));
    }
    let header_date = msg.sent_at.map(|s| s.to_offset(local_offset).date());
    let date = match (body_date, header_date) {
        (Some(d), _) => d,
        (None, Some(d)) => {
            t.date_from_header = true;
            d
        }
        (None, None) => {
            t.result = Err(match &t.date_text {
                Some(text) => TraceFail::DateInvalid(text.clone()),
                None => TraceFail::NoDate,
            });
            return t;
        }
    };
    t.result = t
        .amounts
        .iter()
        .enumerate()
        .map(|(idx, raw)| {
            let amount = normalize_amount(raw)
                .ok()
                .filter(|a| *a > 0)
                .ok_or_else(|| TraceFail::AmountInvalid(raw.clone()))?;
            Ok(Captured {
                idx: u32::try_from(idx).map_err(|_| TraceFail::AmountInvalid(raw.clone()))?,
                amount,
                payee: t.payee.clone(),
                date,
            })
        })
        .collect();
    t
}

impl CompiledRules {
    pub fn new(rules: &[Rule]) -> Self {
        let compiled = rules
            .iter()
            .map(|r| match compile(r) {
                Ok(c) if !c.sender.is_empty() => Some(c),
                Ok(_) => {
                    tracing::warn!(rule = %r.name, "rule email tanpa sender_pattern dilewati");
                    None
                }
                Err(e) => {
                    tracing::warn!(rule = %r.name, error = %e, "regex rule email tidak valid; rule dilewati");
                    None
                }
            })
            .collect();
        Self {
            compiled,
            source: rules.to_vec(),
        }
    }

    /// Rule asal untuk indeks [`Outcome::Parsed::rule`].
    pub fn rule(&self, idx: usize) -> &Rule {
        &self.source[idx]
    }

    pub fn apply(&self, msg: &MailMessage, local_offset: UtcOffset) -> Outcome {
        let from = msg.from.to_lowercase();
        for (i, rule) in self.compiled.iter().enumerate() {
            let Some(rule) = rule else { continue };
            match run(rule, &from, msg, local_offset).result {
                Ok(items) => return Outcome::Parsed { rule: i, items },
                Err(fail) if fail.skips() => continue,
                Err(_) => return Outcome::Unparsed,
            }
        }
        Outcome::Unparsed
    }
}

/// [`CompiledRules::apply`] untuk satu email.
pub fn apply_rules(rules: &[Rule], msg: &MailMessage, local_offset: UtcOffset) -> Outcome {
    CompiledRules::new(rules).apply(msg, local_offset)
}

/// Nominal format Indonesia (`Rp 1.250.000,00`) maupun Inggris (`IDR 41,700.00`) ke satuan minor.
/// Bila `.` dan `,` sama-sama ada, pemisah terakhir adalah desimal; satu `.` yang tidak diikuti
/// tepat 3 digit juga desimal. `,` tanpa `.` hanya diterima sebagai desimal 1–2 digit (`25,00`);
/// selain itu ambigu (`25,000` bisa 25 atau 25.000) dan ditolak agar masuk unparsed.
pub fn normalize_amount(raw: &str) -> Result<i64, AmountParseError> {
    let digits: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || matches!(c, '.' | ','))
        .collect();
    let s = digits.trim_matches(['.', ',']);
    if let (None, Some(comma)) = (s.rfind('.'), s.rfind(','))
        && (s.matches(',').count() > 1 || s.len() - comma - 1 > 2)
    {
        return Err(AmountParseError::Invalid);
    }
    let decimal = match (s.rfind('.'), s.rfind(',')) {
        (Some(dot), Some(comma)) => Some(dot.max(comma)),
        (None, Some(comma)) => Some(comma),
        (Some(dot), None) if s.matches('.').count() == 1 && s.len() - dot - 1 != 3 => Some(dot),
        _ => None,
    };
    let id: String = s
        .char_indices()
        .map(|(i, c)| match c {
            '.' | ',' if Some(i) == decimal => ',',
            '.' | ',' => '.',
            _ => c,
        })
        .collect();
    parse_amount(&id)
}

/// Tanggal dari teks hasil `date_regex`. Teks setelah tanggal (jam, zona) diabaikan.
pub fn parse_date(text: &str, format: &str) -> Option<Date> {
    let text = text.trim();
    let (day, month, rest) = match format {
        "dd/mm/yyyy" | "dd-mm-yyyy" => {
            let sep = if format.contains('/') { '/' } else { '-' };
            let mut parts = text.splitn(3, sep);
            let day = parts.next()?;
            let month = Month::try_from(parts.next()?.parse::<u8>().ok()?).ok()?;
            (day, month, parts.next()?)
        }
        "dd mmm yyyy" => {
            let mut parts = text.split([' ', ',']).filter(|p| !p.is_empty());
            let day = parts.next()?;
            let month = month_from_name(parts.next()?)?;
            (day, month, parts.next()?)
        }
        _ => return None,
    };
    let day: u8 = day.parse().ok()?;
    let year_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    if year_len != 4 {
        return None;
    }
    let year: i32 = rest[..4].parse().ok()?;
    Date::from_calendar_date(year, month, day).ok()
}

/// Nama bulan Indonesia atau Inggris, singkatan maupun lengkap (`Okt`, `Oct`, `October`, `Agustus`).
fn month_from_name(name: &str) -> Option<Month> {
    let name = name.to_lowercase();
    if name.chars().count() < 3 || !name.chars().all(char::is_alphabetic) {
        return None;
    }
    let key: String = name.chars().take(3).collect();
    Some(match key.as_str() {
        "jan" => Month::January,
        "feb" | "peb" => Month::February,
        "mar" => Month::March,
        "apr" => Month::April,
        "mei" | "may" => Month::May,
        "jun" => Month::June,
        "jul" => Month::July,
        "agu" | "aug" => Month::August,
        "sep" => Month::September,
        "okt" | "oct" => Month::October,
        "nov" => Month::November,
        "des" | "dec" => Month::December,
        _ => return None,
    })
}
