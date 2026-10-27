//! Rule bawaan BCA (myBCA) & Jago, dibuat dari format email asli per Oktober 2026.
//!
//! Regex bekerja pada teks hasil [`crate::message::parse_message`]: satu baris tabel
//! menjadi `Label : Nilai` (BCA) atau `Label Nilai` (Jago). Nominal di-anchor ke label
//! agar nominal yang diulang (VA: `billdesc`/`Bill Total`/`Total Payment`) tetap satu item.
use crate::rule::{Rule, RuleKind};

const BCA: &str = "bca.co.id";
const JAGO: &str = "jago.com";
const DMY_NAME: &str = "dd mmm yyyy";
const BCA_DATE: &str = r"^Transaction Date : (\d{1,2} \w+ \d{4})";

struct Def {
    name: &'static str,
    sender: &'static str,
    subject: &'static str,
    amount: &'static str,
    payee: Option<&'static str>,
    date: Option<&'static str>,
    kind: RuleKind,
}

const DEFAULTS: &[Def] = &[
    Def {
        name: "BCA Pembayaran (QRIS/VA)",
        sender: BCA,
        subject: "^Internet Transaction Journal$",
        amount: r"^Total Payment : IDR ([\d.,]+)$",
        payee: Some(r"^(?:Payment to|Company/Product Name) : (.+)$"),
        date: Some(BCA_DATE),
        kind: RuleKind::Expense,
    },
    Def {
        name: "BCA Transfer",
        sender: BCA,
        subject: "^Internet Transaction Journal$",
        amount: r"^Transfer Amount : IDR ([\d.,]+)$",
        payee: Some(r"^Beneficiary Name : (.+)$"),
        date: Some(BCA_DATE),
        kind: RuleKind::Expense,
    },
    Def {
        name: "BCA Tarik Tunai",
        sender: BCA,
        subject: "^Cash Withdrawal Successful$",
        amount: r"^Amount : IDR ([\d.,]+)$",
        payee: None,
        date: Some(BCA_DATE),
        kind: RuleKind::TransferOut,
    },
    Def {
        name: "Jago Pembayaran",
        sender: JAGO,
        subject: "^Kamu telah membayar ke ",
        amount: r"^Jumlah Rp ?([\d.,]+)$",
        payee: Some(r"^Ke (.+?),?$"),
        date: Some(r"^Tanggal Transaksi (\d{1,2} \w+ \d{4})"),
        kind: RuleKind::Expense,
    },
    Def {
        name: "Jago Kartu Debit",
        sender: JAGO,
        subject: "kartu debit Jago$",
        amount: r"sebesar Rp ?([\d.,]+) menggunakan kartu debit",
        payee: None,
        date: None,
        kind: RuleKind::Expense,
    },
    Def {
        name: "Jago Uang Dikembalikan",
        sender: JAGO,
        subject: "^Uang telah dikembalikan$",
        amount: r"sebesar (?:Rp ?)+([\d.,]+) ke ",
        payee: Some(r"sebesar (?:Rp ?)+[\d.,]+ ke (.+?)\.?$"),
        date: None,
        kind: RuleKind::Income,
    },
    Def {
        name: "Jago Terima Uang",
        sender: JAGO,
        subject: "^Kamu telah menerima uang di Jago",
        amount: r"menerima Rp ?([\d.,]+) dari",
        payee: Some(r"menerima Rp ?[\d.,]+ dari akun (.+?)\.?$"),
        date: None,
        kind: RuleKind::TransferIn,
    },
];

/// Rule bawaan tanpa akun/kategori (diisi user nanti), semuanya priority 0.
pub fn default_rules() -> Vec<Rule> {
    DEFAULTS
        .iter()
        .map(|d| Rule {
            id: None,
            name: d.name.into(),
            sender_pattern: d.sender.into(),
            subject_regex: Some(d.subject.into()),
            amount_regex: d.amount.into(),
            payee_regex: d.payee.map(Into::into),
            date_regex: d.date.map(Into::into),
            date_format: d.date.map(|_| DMY_NAME.into()),
            kind: d.kind,
            account_id: None,
            counter_account_id: None,
            default_category_id: None,
            priority: 0,
        })
        .collect()
}
