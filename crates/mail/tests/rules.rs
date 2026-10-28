use nabungo_core::amount::AmountParseError;
use nabungo_mail::defaults::default_rules;
use nabungo_mail::message::{MailMessage, parse_message, raw_message_id};
use nabungo_mail::rule::{
    Captured, CompiledRules, Outcome, Rule, RuleField, RuleKind, TraceFail, apply_rules,
    normalize_amount, parse_date, trace, validate,
};
use time::macros::{date, offset};
use time::{OffsetDateTime, UtcOffset};

const WIB: UtcOffset = offset!(+7);

fn fixture(name: &str) -> MailMessage {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    parse_message(&raw).expect("fixture harus bisa di-parse")
}

/// Hasil satu-item dari rule bawaan: (nama rule, kind, item).
fn parse_single(name: &str) -> (String, RuleKind, Captured) {
    let rules = default_rules();
    match apply_rules(&rules, &fixture(name), WIB) {
        Outcome::Parsed { rule, mut items } => {
            assert_eq!(items.len(), 1, "{name}: {items:?}");
            (rules[rule].name.clone(), rules[rule].kind, items.remove(0))
        }
        Outcome::Unparsed => panic!("{name}: unparsed"),
    }
}

fn rule(name: &str, amount_regex: &str) -> Rule {
    Rule {
        id: None,
        name: name.into(),
        sender_pattern: "example.com".into(),
        subject_regex: None,
        amount_regex: amount_regex.into(),
        payee_regex: None,
        date_regex: None,
        date_format: None,
        kind: RuleKind::Expense,
        account_id: None,
        counter_account_id: None,
        default_category_id: None,
        priority: 0,
    }
}

fn plain(subject: &str, date_header: &str, body: &str) -> MailMessage {
    let raw = format!(
        "Message-ID: <x@example.com>\r\nFrom: Bank <notif@example.com>\r\nSubject: {subject}\r\n\
         Date: {date_header}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{body}\r\n"
    );
    parse_message(raw.as_bytes()).unwrap()
}

#[test]
fn bca_qris_is_expense() {
    let (_, kind, item) = parse_single("bca_qris.eml");
    assert_eq!(kind, RuleKind::Expense);
    assert_eq!(item.idx, 0);
    assert_eq!(item.amount, 41_700);
    assert_eq!(item.payee.as_deref(), Some("IDM INDOMARET D"));
    assert_eq!(item.date, date!(2026 - 05 - 09));
}

#[test]
fn bca_transfer_is_expense() {
    let (_, kind, item) = parse_single("bca_transfer.eml");
    assert_eq!(kind, RuleKind::Expense);
    assert_eq!(item.amount, 10_000);
    assert_eq!(item.payee.as_deref(), Some("ANDI WIJAYA"));
    assert_eq!(item.date, date!(2026 - 05 - 19));
}

#[test]
fn bca_va_repeated_amount_is_one_item() {
    let (_, kind, item) = parse_single("bca_va.eml");
    assert_eq!(kind, RuleKind::Expense);
    assert_eq!(item.amount, 33_300);
    assert_eq!(item.payee.as_deref(), Some("PT TOKOPEDIA / TOKOPEDIA"));
    assert_eq!(item.date, date!(2026 - 04 - 03));
}

#[test]
fn bca_cash_withdrawal_is_transfer_out() {
    let (_, kind, item) = parse_single("bca_tarik_tunai.eml");
    assert_eq!(kind, RuleKind::TransferOut);
    assert_eq!(item.amount, 500_000);
    assert_eq!(item.payee, None);
    assert_eq!(item.date, date!(2026 - 05 - 16));
}

#[test]
fn jago_payment_is_expense() {
    let (_, kind, item) = parse_single("jago_bayar.eml");
    assert_eq!(kind, RuleKind::Expense);
    assert_eq!(item.amount, 21_000);
    assert_eq!(item.payee.as_deref(), Some("Lazatto Chicken & Burger"));
    assert_eq!(item.date, date!(2026 - 05 - 15));
}

#[test]
fn jago_debit_card_uses_header_date() {
    let (_, kind, item) = parse_single("jago_debit.eml");
    assert_eq!(kind, RuleKind::Expense);
    assert_eq!(item.amount, 500);
    assert_eq!(item.payee, None);
    // Date: Tue, 26 May 2026 17:30:00 GMT → 27 Mei di +07:00.
    assert_eq!(item.date, date!(2026 - 05 - 27));
}

#[test]
fn jago_refund_is_income() {
    let (_, kind, item) = parse_single("jago_dikembalikan.eml");
    assert_eq!(kind, RuleKind::Income);
    assert_eq!(item.amount, 500);
    assert_eq!(
        item.payee.as_deref(),
        Some("DEI*AWS ID NON3DS JAKARTA SELAT ID")
    );
    assert_eq!(item.date, date!(2026 - 07 - 22));
}

#[test]
fn jago_receive_gopay_is_transfer_in() {
    let (_, kind, item) = parse_single("jago_terima_gopay.eml");
    assert_eq!(kind, RuleKind::TransferIn);
    assert_eq!(item.amount, 100_000);
    assert_eq!(item.payee.as_deref(), Some("Gopay"));
    assert_eq!(item.date, date!(2026 - 06 - 01));
}

#[test]
fn fractional_amount_goes_unparsed() {
    let out = apply_rules(&default_rules(), &fixture("bca_fractional.eml"), WIB);
    assert_eq!(out, Outcome::Unparsed);
}

#[test]
fn unknown_sender_goes_unparsed() {
    let out = apply_rules(&default_rules(), &fixture("unknown_sender.eml"), WIB);
    assert_eq!(out, Outcome::Unparsed);
}

#[test]
fn zero_amount_goes_unparsed() {
    let msg = plain("x", "Mon, 26 Oct 2026 10:00:00 +0700", "Total Rp 0");
    let out = apply_rules(&[rule("nol", r"^Total Rp ([\d.,]+)$")], &msg, WIB);
    assert_eq!(out, Outcome::Unparsed);
}

#[test]
fn multiple_amounts_get_indexes() {
    let msg = plain(
        "Mutasi",
        "Mon, 26 Oct 2026 10:00:00 +0700",
        "Debet Rp 10.000\nDebet Rp 2.500",
    );
    match apply_rules(&[rule("mutasi", r"^Debet Rp ([\d.,]+)$")], &msg, WIB) {
        Outcome::Parsed { rule, items } => {
            assert_eq!(rule, 0);
            let got: Vec<(u32, i64)> = items.iter().map(|c| (c.idx, c.amount)).collect();
            assert_eq!(got, vec![(0, 10_000), (1, 2_500)]);
        }
        Outcome::Unparsed => panic!("unparsed"),
    }
}

#[test]
fn date_falls_back_to_header() {
    let msg = plain("x", "Mon, 26 Oct 2026 20:30:00 +0000", "Total Rp 5.000");
    let mut r = rule("tanpa tanggal", r"^Total Rp ([\d.,]+)$");
    // date_regex yang tidak cocok juga jatuh ke header.
    r.date_regex = Some(r"^Tanggal (.+)$".into());
    r.date_format = Some("dd mmm yyyy".into());
    match apply_rules(&[r], &msg, WIB) {
        Outcome::Parsed { items, .. } => assert_eq!(items[0].date, date!(2026 - 10 - 27)),
        Outcome::Unparsed => panic!("unparsed"),
    }
}

#[test]
fn invalid_regex_rule_is_skipped() {
    let msg = plain("x", "Mon, 26 Oct 2026 10:00:00 +0700", "Total Rp 5.000");
    let rules = [rule("rusak", "("), rule("benar", r"^Total Rp ([\d.,]+)$")];
    match apply_rules(&rules, &msg, WIB) {
        Outcome::Parsed { rule, .. } => assert_eq!(rule, 1),
        Outcome::Unparsed => panic!("unparsed"),
    }
}

#[test]
fn subject_regex_must_match() {
    let msg = plain("Promo", "Mon, 26 Oct 2026 10:00:00 +0700", "Total Rp 5.000");
    let mut r = rule("bersubjek", r"^Total Rp ([\d.,]+)$");
    r.subject_regex = Some("^Transaksi".into());
    assert_eq!(apply_rules(&[r], &msg, WIB), Outcome::Unparsed);
}

#[test]
fn payee_from_body_and_trimmed() {
    let msg = plain(
        "x",
        "Mon, 26 Oct 2026 10:00:00 +0700",
        "Ke    Warung Bu Sri  \nTotal Rp 5.000",
    );
    let mut r = rule("payee", r"^Total Rp ([\d.,]+)$");
    r.payee_regex = Some(r"^Ke (.*)$".into());
    match apply_rules(&[r], &msg, WIB) {
        Outcome::Parsed { items, .. } => {
            assert_eq!(items[0].payee.as_deref(), Some("Warung Bu Sri"))
        }
        Outcome::Unparsed => panic!("unparsed"),
    }
}

#[test]
fn month_names_id_and_en() {
    let f = "dd mmm yyyy";
    assert_eq!(parse_date("27 Okt 2026", f), Some(date!(2026 - 10 - 27)));
    assert_eq!(parse_date("27 oct 2026", f), Some(date!(2026 - 10 - 27)));
    assert_eq!(parse_date("3 Mei 2026", f), Some(date!(2026 - 05 - 03)));
    assert_eq!(parse_date("17 Agu 2026", f), Some(date!(2026 - 08 - 17)));
    assert_eq!(parse_date("1 Agustus 2026", f), Some(date!(2026 - 08 - 01)));
    assert_eq!(parse_date("25 Des 2026", f), Some(date!(2026 - 12 - 25)));
    assert_eq!(
        parse_date("26 October 2026, 18:26 WIB", f),
        Some(date!(2026 - 10 - 26))
    );
    assert_eq!(
        parse_date("09 May 2026 11:14:28", f),
        Some(date!(2026 - 05 - 09))
    );
    assert_eq!(parse_date("31 Feb 2026", f), None);
    assert_eq!(parse_date("27 Foo 2026", f), None);
}

#[test]
fn numeric_date_formats() {
    assert_eq!(
        parse_date("27/10/2026", "dd/mm/yyyy"),
        Some(date!(2026 - 10 - 27))
    );
    assert_eq!(
        parse_date("03-05-2026", "dd-mm-yyyy"),
        Some(date!(2026 - 05 - 03))
    );
    assert_eq!(parse_date("27/10/2026", "dd-mm-yyyy"), None);
    assert_eq!(parse_date("27/10/2026", "yyyy"), None);
}

#[test]
fn normalize_amount_handles_id_and_en_formats() {
    assert_eq!(normalize_amount("IDR 41,700.00"), Ok(41_700));
    assert_eq!(normalize_amount("IDR 3,000,000.00"), Ok(3_000_000));
    assert_eq!(normalize_amount("Rp 21.000"), Ok(21_000));
    assert_eq!(normalize_amount("Rp Rp500"), Ok(500));
    assert_eq!(normalize_amount("Rp 1.250.000,00"), Ok(1_250_000));
    assert_eq!(
        normalize_amount("Rp 25.000,50"),
        Err(AmountParseError::Fractional)
    );
    assert_eq!(
        normalize_amount("IDR 25,000.50"),
        Err(AmountParseError::Fractional)
    );
    // Satu koma tanpa titik dibaca desimal: lebih baik unparsed daripada salah nominal.
    assert!(normalize_amount("1,500").is_err());
    // Koma diikuti tepat 3 digit = pemisah ribuan yang ambigu: tolak, jangan dibaca 25 / 1500.
    assert!(normalize_amount("IDR 25,000").is_err());
    assert!(normalize_amount("IDR 1,500,000").is_err());
    assert!(normalize_amount("1,000").is_err());
    assert_eq!(normalize_amount("Rp 25,00"), Ok(25));
    assert!(normalize_amount("20.5").is_err());
    assert_eq!(normalize_amount("Rp"), Err(AmountParseError::Empty));
}

#[test]
fn compiled_rules_expose_source_rule() {
    let rules = default_rules();
    let compiled = CompiledRules::new(&rules);
    match compiled.apply(&fixture("bca_tarik_tunai.eml"), WIB) {
        Outcome::Parsed { rule, .. } => assert_eq!(compiled.rule(rule), &rules[rule]),
        Outcome::Unparsed => panic!("unparsed"),
    }
}

#[test]
fn raw_message_id_is_hash_of_raw() {
    let id = raw_message_id(b"bukan email");
    assert!(id.starts_with("sha256:"), "{id}");
    assert_eq!(id, raw_message_id(b"bukan email"));
    assert_ne!(id, raw_message_id(b"lain"));
}

fn bare(from: &str, subject: &str, text: &str) -> MailMessage {
    MailMessage {
        message_id: "m@example.com".into(),
        from: from.into(),
        subject: subject.into(),
        sent_at: None,
        text: text.into(),
    }
}

#[test]
fn validate_accepts_default_rules() {
    for r in default_rules() {
        assert_eq!(validate(&r), Ok(()), "{}", r.name);
    }
}

#[test]
fn validate_rejects_empty_name_and_sender() {
    let mut r = rule("  ", r"Rp ([\d.]+)");
    assert_eq!(validate(&r), Err(RuleField::Name));
    r.name = "ok".into();
    r.sender_pattern = " ".into();
    assert_eq!(validate(&r), Err(RuleField::Sender));
}

#[test]
fn validate_rejects_bad_regex() {
    assert_eq!(validate(&rule("x", "(")), Err(RuleField::Amount));
    let mut r = rule("x", r"Rp ([\d.]+)");
    r.subject_regex = Some("[".into());
    assert_eq!(validate(&r), Err(RuleField::Subject));
}

#[test]
fn validate_rejects_amount_without_group() {
    assert_eq!(validate(&rule("x", r"Rp [\d.]+")), Err(RuleField::Amount));
}

#[test]
fn validate_rejects_optional_regex_without_group() {
    let mut r = rule("x", r"Rp ([\d.]+)");
    r.payee_regex = Some("^Ke .+$".into());
    assert_eq!(validate(&r), Err(RuleField::Payee));
}

#[test]
fn validate_rejects_date_without_format() {
    let mut r = rule("x", r"Rp ([\d.]+)");
    r.date_regex = Some(r"^Tgl (.+)$".into());
    assert_eq!(validate(&r), Err(RuleField::DateFormat));
    r.date_regex = None;
    r.date_format = Some("dd/mm/yyyy".into());
    assert_eq!(validate(&r), Err(RuleField::Date));
}

#[test]
fn validate_rejects_unknown_date_format() {
    let mut r = rule("x", r"Rp ([\d.]+)");
    r.date_regex = Some(r"^Tgl (.+)$".into());
    r.date_format = Some("yyyy-mm-dd".into());
    assert_eq!(validate(&r), Err(RuleField::DateFormat));
}

#[test]
fn trace_invalid_rule_is_err() {
    let msg = bare("a@example.com", "x", "Rp 5.000");
    assert_eq!(
        trace(&rule("x", "("), &msg, WIB).err(),
        Some(RuleField::Amount)
    );
}

#[test]
fn trace_reports_failing_step() {
    let r = rule("x", r"^Total Rp ([\d.,]+)$");
    let t = trace(&r, &bare("a@other.com", "x", "Total Rp 5.000"), WIB).unwrap();
    assert!(!t.sender_ok);
    assert_eq!(t.result, Err(TraceFail::Sender));

    let mut s = r.clone();
    s.subject_regex = Some("^Transaksi".into());
    let t = trace(&s, &bare("a@example.com", "Promo", "Total Rp 5.000"), WIB).unwrap();
    assert!(t.sender_ok);
    assert_eq!(t.subject_ok, Some(false));
    assert_eq!(t.result, Err(TraceFail::Subject));

    let t = trace(&r, &bare("a@example.com", "x", "Halo"), WIB).unwrap();
    assert_eq!(t.subject_ok, None);
    assert_eq!(t.result, Err(TraceFail::NoAmount));

    let mut dated = bare("a@example.com", "x", "Total Rp 25,000");
    dated.sent_at = Some(OffsetDateTime::from_unix_timestamp(1_793_000_000).unwrap());
    let t = trace(&r, &dated, WIB).unwrap();
    assert_eq!(t.amounts, vec!["25,000".to_string()]);
    assert_eq!(t.result, Err(TraceFail::AmountInvalid("25,000".into())));

    // Tanpa header Date: tanggal wajib dari body.
    let t = trace(&r, &bare("a@example.com", "x", "Total Rp 5.000"), WIB).unwrap();
    assert_eq!(t.result, Err(TraceFail::NoDate));

    let mut d = r.clone();
    d.date_regex = Some(r"^Tanggal (.+)$".into());
    d.date_format = Some("dd mmm yyyy".into());
    let msg = bare("a@example.com", "x", "Tanggal 32 Okt 2026\nTotal Rp 5.000");
    let t = trace(&d, &msg, WIB).unwrap();
    assert_eq!(t.date_text.as_deref(), Some("32 Okt 2026"));
    assert_eq!(t.result, Err(TraceFail::DateInvalid("32 Okt 2026".into())));
}

#[test]
fn trace_date_invalid_falls_back_to_header() {
    let mut d = rule("x", r"^Total Rp ([\d.,]+)$");
    d.date_regex = Some(r"^Tanggal (.+)$".into());
    d.date_format = Some("dd mmm yyyy".into());
    let mut msg = bare("a@example.com", "x", "Tanggal 32 Okt 2026\nTotal Rp 5.000");
    msg.sent_at = Some(OffsetDateTime::from_unix_timestamp(1_793_000_000).unwrap());
    let t = trace(&d, &msg, WIB).unwrap();
    assert!(t.date_from_header);
    let items = t.result.unwrap();
    assert_eq!(items[0].amount, 5_000);
    assert_eq!(
        apply_rules(&[d], &msg, WIB),
        Outcome::Parsed { rule: 0, items }
    );
}

#[test]
fn trace_matches_apply_on_fixtures() {
    let rules = default_rules();
    let dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("eml") {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        let msg = fixture(&name);
        let traces: Vec<_> = rules.iter().map(|r| trace(r, &msg, WIB).unwrap()).collect();
        let chosen = traces.iter().position(|t| {
            !matches!(
                t.result,
                Err(TraceFail::Sender | TraceFail::Subject | TraceFail::NoAmount)
            )
        });
        let expected = match chosen {
            Some(i) => match &traces[i].result {
                Ok(items) => Outcome::Parsed {
                    rule: i,
                    items: items.clone(),
                },
                Err(_) => Outcome::Unparsed,
            },
            None => Outcome::Unparsed,
        };
        assert_eq!(apply_rules(&rules, &msg, WIB), expected, "{name}");
    }
}
