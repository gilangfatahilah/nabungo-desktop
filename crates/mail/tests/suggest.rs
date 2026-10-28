use nabungo_mail::message::{MailMessage, parse_message};
use nabungo_mail::rule::{RuleKind, trace, validate};
use nabungo_mail::suggest::suggest;
use time::UtcOffset;
use time::macros::{date, offset};

const WIB: UtcOffset = offset!(+7);

fn fixture(name: &str) -> MailMessage {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    parse_message(&std::fs::read(path).unwrap()).unwrap()
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
fn suggest_from_bca_qris() {
    let msg = fixture("bca_qris.eml");
    let r = suggest(&msg);
    assert_eq!(validate(&r), Ok(()));
    assert_eq!(r.sender_pattern, "bca.co.id");
    assert_eq!(
        r.subject_regex.as_deref(),
        Some("^Internet Transaction Journal$")
    );
    assert_eq!(r.date_format.as_deref(), Some("dd mmm yyyy"));
    assert_eq!(r.kind, RuleKind::Expense);
    assert_eq!(r.priority, 10);
    let t = trace(&r, &msg, WIB).unwrap();
    assert!(!t.date_from_header);
    let items = t.result.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].amount, 41_700);
    assert_eq!(items[0].date, date!(2026 - 05 - 09));
}

#[test]
fn suggest_falls_back_to_generic_amount() {
    let mut msg = bare(
        "notif@toko.example",
        "Bayar",
        "Anda membayar Rp 15.000 di toko 7",
    );
    msg.sent_at = Some(time::OffsetDateTime::from_unix_timestamp(1_793_000_000).unwrap());
    let r = suggest(&msg);
    assert_eq!(validate(&r), Ok(()));
    assert_eq!(r.sender_pattern, "toko.example");
    let items = trace(&r, &msg, WIB).unwrap().result.unwrap();
    assert_eq!(items[0].amount, 15_000);
}

#[test]
fn suggest_escapes_subject() {
    let mut msg = bare(
        "a@b.example",
        "Transaksi (QRIS) sukses?",
        "Total : Rp 5.000",
    );
    msg.sent_at = Some(time::OffsetDateTime::from_unix_timestamp(1_793_000_000).unwrap());
    let r = suggest(&msg);
    assert_eq!(validate(&r), Ok(()));
    let t = trace(&r, &msg, WIB).unwrap();
    assert_eq!(t.subject_ok, Some(true));
    assert_eq!(t.result.unwrap()[0].amount, 5_000);
}

#[test]
fn suggest_name_truncates() {
    let subject = "x".repeat(100);
    let r = suggest(&bare("a@b.example", &subject, "Rp 5.000"));
    assert!(r.name.chars().count() <= 60);
    assert!(!r.name.is_empty());
}

#[test]
fn suggest_without_subject_uses_domain_as_name() {
    let r = suggest(&bare("a@b.example", "", "Rp 5.000"));
    assert_eq!(r.name, "b.example");
    assert_eq!(r.subject_regex, None);
}

/// Kasus yang sama ada di `ui/src/ipc/browser-rule.test.ts`.
#[test]
fn suggest_anchors_labels_from_text() {
    let msg = bare(
        "Notif@bca.co.id",
        "Internet Transaction Journal",
        "Transaction Date : 09 May 2026 12:00:00\nPayment to : IDM\nTotal Payment : IDR 41,700.00\n",
    );
    let r = suggest(&msg);
    assert_eq!(r.name, "Internet Transaction Journal");
    assert_eq!(r.sender_pattern, "bca.co.id");
    assert_eq!(r.amount_regex, r"^Total Payment : IDR ?([\d.,]+)$");
    assert_eq!(
        r.date_regex.as_deref(),
        Some(r"^Transaction Date : (\d{1,2} \w+ \d{4})")
    );
    let t = trace(&r, &msg, WIB).unwrap();
    assert_eq!(t.result.unwrap()[0].amount, 41_700);
}
