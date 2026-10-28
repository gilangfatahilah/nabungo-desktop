//! Paritas rule engine: kasus yang sama dijalankan `ui/src/ipc/browser-rule.test.ts`.
use nabungo_mail::message::MailMessage;
use nabungo_mail::rule::{Rule, RuleKind, TraceFail, trace};
use serde::Deserialize;
use time::OffsetDateTime;
use time::macros::offset;

#[derive(Deserialize)]
struct Cases {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    rule: CaseRule,
    email: CaseEmail,
    expect: Expect,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaseRule {
    sender: String,
    subject: Option<String>,
    amount: String,
    payee: Option<String>,
    date: Option<String>,
    date_format: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaseEmail {
    from: String,
    subject: String,
    sent_at: Option<i64>,
    text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Expect {
    sender_ok: bool,
    subject_ok: Option<bool>,
    amounts: Vec<String>,
    payee: Option<String>,
    date_text: Option<String>,
    date_from_header: bool,
    fail: Option<String>,
    fail_text: Option<String>,
    items: Vec<Item>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Item {
    amount: i64,
    date: String,
    payee: Option<String>,
}

fn fail_of(f: &TraceFail) -> (&'static str, Option<String>) {
    match f {
        TraceFail::Sender => ("sender", None),
        TraceFail::Subject => ("subject", None),
        TraceFail::NoAmount => ("no_amount", None),
        TraceFail::AmountInvalid(t) => ("amount_invalid", Some(t.clone())),
        TraceFail::NoDate => ("no_date", None),
        TraceFail::DateInvalid(t) => ("date_invalid", Some(t.clone())),
    }
}

#[test]
fn rule_cases_parity() {
    let path = format!(
        "{}/tests/fixtures/rule-cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let cases: Cases = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert!(cases.cases.len() >= 12);
    for c in cases.cases {
        let rule = Rule {
            id: None,
            name: c.name.clone(),
            sender_pattern: c.rule.sender,
            subject_regex: c.rule.subject,
            amount_regex: c.rule.amount,
            payee_regex: c.rule.payee,
            date_regex: c.rule.date,
            date_format: c.rule.date_format,
            kind: RuleKind::Expense,
            account_id: None,
            counter_account_id: None,
            default_category_id: None,
            priority: 0,
        };
        let msg = MailMessage {
            message_id: "m@example.com".into(),
            from: c.email.from,
            subject: c.email.subject,
            sent_at: c
                .email
                .sent_at
                .map(|s| OffsetDateTime::from_unix_timestamp(s).unwrap()),
            text: c.email.text,
        };
        let t = trace(&rule, &msg, offset!(+7)).expect(&c.name);
        let e = c.expect;
        let n = &c.name;
        assert_eq!(t.sender_ok, e.sender_ok, "{n}");
        assert_eq!(t.subject_ok, e.subject_ok, "{n}");
        assert_eq!(t.amounts, e.amounts, "{n}");
        assert_eq!(t.payee, e.payee, "{n}");
        assert_eq!(t.date_text, e.date_text, "{n}");
        assert_eq!(t.date_from_header, e.date_from_header, "{n}");
        match &t.result {
            Ok(items) => {
                assert_eq!(e.fail, None, "{n}");
                let got: Vec<Item> = items
                    .iter()
                    .map(|i| Item {
                        amount: i.amount,
                        date: i.date.to_string(),
                        payee: i.payee.clone(),
                    })
                    .collect();
                assert_eq!(got, e.items, "{n}");
            }
            Err(f) => {
                let (code, text) = fail_of(f);
                assert_eq!(Some(code), e.fail.as_deref(), "{n}");
                assert_eq!(text, e.fail_text, "{n}");
                assert!(e.items.is_empty(), "{n}");
            }
        }
    }
}
