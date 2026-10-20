//! Ekspor data (DAT-02): satu file JSON lengkap, atau folder CSV untuk spreadsheet.

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use nabungo_storage::export::{ExportTxn, for_each_txn};
use nabungo_storage::{account, account_group, budget, category};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime};

use crate::{AccountRow, AppError, CategoryRow, Ledger, Uuid};

const FORMAT: &str = "nabungo-export";
const VERSION: u32 = 1;
const BOM: &[u8] = b"\xEF\xBB\xBF";

#[derive(Serialize)]
struct JsonDoc<'a> {
    format: &'static str,
    version: u32,
    exported_at: String,
    app_version: &'static str,
    accounts: Vec<JsonAccount<'a>>,
    categories: Vec<JsonCategory<'a>>,
    budgets: Vec<JsonBudget<'a>>,
    transactions: Vec<JsonTxn>,
}

#[derive(Serialize)]
struct JsonAccount<'a> {
    id: Uuid,
    name: &'a str,
    kind: &'static str,
    group_id: Option<Uuid>,
    currency: &'a str,
    include_in_net_worth: bool,
    credit_limit: Option<i64>,
    statement_day: Option<u8>,
    archived: bool,
    /// Dihitung dari postings saat ekspor (K15).
    balance: i64,
}

#[derive(Serialize)]
struct JsonCategory<'a> {
    id: Uuid,
    parent_id: Option<Uuid>,
    kind: &'static str,
    name: &'a str,
    archived: bool,
}

#[derive(Serialize)]
struct JsonBudget<'a> {
    category_id: Uuid,
    period: &'a str,
    amount: i64,
}

#[derive(Serialize)]
struct JsonTxn {
    id: Uuid,
    kind: String,
    date: String,
    status: String,
    payee: Option<String>,
    note: Option<String>,
    tags: Vec<String>,
    postings: Vec<JsonPosting>,
}

#[derive(Serialize)]
struct JsonPosting {
    account_id: Uuid,
    category_id: Option<Uuid>,
    amount: i64,
    currency: String,
    memo: Option<String>,
}

impl From<ExportTxn> for JsonTxn {
    fn from(t: ExportTxn) -> Self {
        Self {
            id: t.id,
            kind: t.kind,
            date: t.date,
            status: t.status,
            payee: t.payee,
            note: t.note,
            tags: t.tags,
            postings: t
                .postings
                .into_iter()
                .map(|p| JsonPosting {
                    account_id: p.account_id,
                    category_id: p.category_id,
                    amount: p.amount,
                    currency: p.currency,
                    memo: p.memo,
                })
                .collect(),
        }
    }
}

/// K14: sel teks yang bisa dibaca spreadsheet sebagai formula diberi awalan `'`.
fn cell(s: &str) -> Cow<'_, str> {
    if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        Cow::Owned(format!("'{s}"))
    } else {
        Cow::Borrowed(s)
    }
}

fn opt(s: &Option<String>) -> Cow<'_, str> {
    cell(s.as_deref().unwrap_or(""))
}

/// `Parent / Child` untuk kategori child (K13).
fn category_paths(categories: &[CategoryRow]) -> HashMap<Uuid, String> {
    let by_id: HashMap<_, _> = categories.iter().map(|c| (c.id, c)).collect();
    categories
        .iter()
        .map(|c| {
            let path = match c.parent_id.and_then(|p| by_id.get(&p)) {
                Some(parent) => format!("{} / {}", parent.name, c.name),
                None => c.name.clone(),
            };
            (c.id, path)
        })
        .collect()
}

fn csv_writer(path: &Path) -> Result<csv::Writer<std::fs::File>, AppError> {
    let mut file = std::fs::File::create_new(path)?;
    file.write_all(BOM)?;
    Ok(csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(file))
}

fn csv_err(e: csv::Error) -> AppError {
    match e.into_kind() {
        csv::ErrorKind::Io(e) => AppError::Io(e),
        other => AppError::Io(std::io::Error::other(format!("{other:?}"))),
    }
}

/// Folder `nabungo-export-YYYY-MM-DD` baru; tidak pernah memakai folder yang sudah ada.
fn create_export_dir(parent: &Path, today: Date) -> Result<PathBuf, AppError> {
    let base = format!("nabungo-export-{}", crate::format_date(today));
    for n in 1u32.. {
        let name = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        let dir = parent.join(name);
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!("u32 suffix exhausted")
}

fn bool_cell(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

impl Ledger {
    /// Ekspor lengkap ke satu dokumen JSON (K12). Hanya membaca DB.
    pub fn export_json(&self, w: &mut impl Write, now: OffsetDateTime) -> Result<(), AppError> {
        let accounts = account::list_accounts(&self.conn)?;
        let categories = category::list_categories(&self.conn)?;
        let budgets = budget::list_all_budgets(&self.conn)?;
        let mut transactions = Vec::new();
        for_each_txn(&self.conn, |t| {
            transactions.push(JsonTxn::from(t));
            Ok::<_, AppError>(())
        })?;
        let doc = JsonDoc {
            format: FORMAT,
            version: VERSION,
            exported_at: now
                .to_offset(time::UtcOffset::UTC)
                .replace_nanosecond(0)
                .unwrap_or(now)
                .format(&Rfc3339)
                .map_err(|e| std::io::Error::other(e.to_string()))?,
            app_version: crate::version(),
            accounts: accounts.iter().map(json_account).collect(),
            categories: categories
                .iter()
                .map(|c| JsonCategory {
                    id: c.id,
                    parent_id: c.parent_id,
                    kind: c.kind.as_str(),
                    name: &c.name,
                    archived: c.archived,
                })
                .collect(),
            budgets: budgets
                .iter()
                .map(|b| JsonBudget {
                    category_id: b.category_id,
                    period: &b.period,
                    amount: b.amount,
                })
                .collect(),
            transactions,
        };
        serde_json::to_writer_pretty(&mut *w, &doc).map_err(std::io::Error::from)?;
        w.flush()?;
        Ok(())
    }

    /// Ekspor ke folder CSV baru di `parent` (K13–K15); mengembalikan folder itu.
    /// Gagal di tengah jalan → folder yang baru dibuat dihapus.
    pub fn export_csv(&self, parent: &Path, today: Date) -> Result<PathBuf, AppError> {
        let dir = create_export_dir(parent, today)?;
        match self.write_csv_files(&dir) {
            Ok(()) => Ok(dir),
            Err(e) => {
                let _ = std::fs::remove_dir_all(&dir);
                Err(e)
            }
        }
    }

    fn write_csv_files(&self, dir: &Path) -> Result<(), AppError> {
        let accounts = account::list_accounts(&self.conn)?;
        let groups: HashMap<_, _> = account_group::list_groups(&self.conn)?
            .into_iter()
            .map(|g| (g.id, g.name))
            .collect();
        let categories = category::list_categories(&self.conn)?;
        let paths = category_paths(&categories);
        let account_names: HashMap<_, _> =
            accounts.iter().map(|a| (a.id, a.name.as_str())).collect();
        let name_of = |id: &Uuid| paths.get(id).map_or("", String::as_str);

        let mut w = csv_writer(&dir.join("accounts.csv"))?;
        w.write_record([
            "id",
            "name",
            "kind",
            "group",
            "currency",
            "archived",
            "include_in_net_worth",
            "balance",
        ])
        .map_err(csv_err)?;
        for a in &accounts {
            let group = a
                .group_id
                .and_then(|g| groups.get(&g))
                .map_or("", String::as_str);
            w.write_record([
                a.id.to_string().as_str(),
                &cell(&a.name),
                a.kind.as_str(),
                &cell(group),
                a.currency.as_str(),
                bool_cell(a.archived),
                bool_cell(a.include_in_net_worth),
                &a.balance.to_string(),
            ])
            .map_err(csv_err)?;
        }
        w.flush()?;

        let mut w = csv_writer(&dir.join("categories.csv"))?;
        w.write_record(["id", "name", "kind", "parent", "archived"])
            .map_err(csv_err)?;
        for c in &categories {
            let parent = c.parent_id.as_ref().map_or("", name_of);
            w.write_record([
                c.id.to_string().as_str(),
                &cell(&c.name),
                c.kind.as_str(),
                &cell(parent),
                bool_cell(c.archived),
            ])
            .map_err(csv_err)?;
        }
        w.flush()?;

        let mut w = csv_writer(&dir.join("budgets.csv"))?;
        w.write_record(["period", "category", "amount"])
            .map_err(csv_err)?;
        for b in budget::list_all_budgets(&self.conn)? {
            w.write_record([
                cell(&b.period).as_ref(),
                &cell(name_of(&b.category_id)),
                &b.amount.to_string(),
            ])
            .map_err(csv_err)?;
        }
        w.flush()?;

        let mut w = csv_writer(&dir.join("transactions.csv"))?;
        w.write_record([
            "txn_id", "date", "kind", "status", "payee", "note", "tags", "account", "category",
            "amount", "currency", "memo",
        ])
        .map_err(csv_err)?;
        for_each_txn(&self.conn, |t| {
            let id = t.id.to_string();
            let tags = t.tags.join(";");
            for p in &t.postings {
                let account = account_names.get(&p.account_id).copied().unwrap_or("");
                let category = p.category_id.as_ref().map_or("", name_of);
                w.write_record([
                    id.as_str(),
                    &cell(&t.date),
                    &cell(&t.kind),
                    &cell(&t.status),
                    &opt(&t.payee),
                    &opt(&t.note),
                    &cell(&tags),
                    &cell(account),
                    &cell(category),
                    &p.amount.to_string(),
                    &cell(&p.currency),
                    &opt(&p.memo),
                ])
                .map_err(csv_err)?;
            }
            Ok::<_, AppError>(())
        })?;
        w.flush()?;
        Ok(())
    }
}

fn json_account(a: &AccountRow) -> JsonAccount<'_> {
    JsonAccount {
        id: a.id,
        name: &a.name,
        kind: a.kind.as_str(),
        group_id: a.group_id,
        currency: a.currency.as_str(),
        include_in_net_worth: a.include_in_net_worth,
        credit_limit: a.credit_limit,
        statement_day: a.statement_day,
        archived: a.archived,
        balance: a.balance,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nabungo_core::{BudgetPeriod, CategoryKind, FormBody, SplitLine};
    use time::macros::{date, datetime};

    use crate::testkit::Kit;
    use crate::{CategoryFields, TxnForm};

    const NOW: time::OffsetDateTime = datetime!(2026-10-20 03:15:00 UTC);
    const TODAY: time::Date = date!(2026 - 10 - 20);

    /// Expense, transfer + biaya, income, satu transaksi terhapus, budget.
    fn seeded() -> Kit {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        k.expense(date!(2026 - 10 - 02), "Makan & Minum", 25_000);
        k.transfer(date!(2026 - 10 - 03), 100_000, Some(2_500));
        k.income(date!(2026 - 10 - 04), "Gaji", 5_000_000);
        let gone = k.expense(date!(2026 - 10 - 05), "Makan & Minum", 9_999);
        k.ledger.delete_txn(gone).unwrap();
        let makan = k.cat("Makan & Minum");
        k.ledger
            .set_budget(makan, BudgetPeriod::new(2026, 10).unwrap(), Some(500_000))
            .unwrap();
        k
    }

    fn json(k: &Kit) -> serde_json::Value {
        let mut buf = Vec::new();
        k.ledger.export_json(&mut buf, NOW).unwrap();
        serde_json::from_slice(&buf).unwrap()
    }

    fn read(path: &Path) -> String {
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(&bytes[..3], b"\xEF\xBB\xBF", "{path:?} tanpa BOM");
        String::from_utf8(bytes[3..].to_vec()).unwrap()
    }

    fn records(path: &Path) -> Vec<csv::StringRecord> {
        csv::Reader::from_reader(read(path).as_bytes())
            .records()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn json_export_has_all_collections() {
        let k = seeded();
        let v = json(&k);
        assert_eq!(v["format"], "nabungo-export");
        assert_eq!(v["version"], 1);
        assert_eq!(v["exported_at"], "2026-10-20T03:15:00Z");
        assert_eq!(v["app_version"], crate::version());
        let txns = v["transactions"].as_array().unwrap();
        // 1 opening + expense + transfer + income; yang terhapus tidak ikut.
        assert_eq!(txns.len(), 4);
        let transfer = txns.iter().find(|t| t["kind"] == "transfer").unwrap();
        let sides: Vec<_> = transfer["postings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["category_id"].is_null())
            .collect();
        assert_eq!(sides.len(), 2);
        assert!(sides.iter().all(|p| p["currency"] == "IDR"));
        for a in v["accounts"].as_array().unwrap() {
            let id: crate::Uuid = a["id"].as_str().unwrap().parse().unwrap();
            assert_eq!(a["balance"], k.ledger.balance(id).unwrap());
        }
        assert!(!v["categories"].as_array().unwrap().is_empty());
        let budgets = v["budgets"].as_array().unwrap();
        assert_eq!(budgets.len(), 1);
        assert_eq!(budgets[0]["period"], "2026-10");
        assert_eq!(budgets[0]["amount"], 500_000);
    }

    #[test]
    fn csv_export_creates_new_folder_each_time() {
        let k = seeded();
        let dir = tempfile::tempdir().unwrap();
        let first = k.ledger.export_csv(dir.path(), TODAY).unwrap();
        assert_eq!(first, dir.path().join("nabungo-export-2026-10-20"));
        let before = std::fs::read(first.join("transactions.csv")).unwrap();
        let second = k.ledger.export_csv(dir.path(), TODAY).unwrap();
        assert_eq!(second, dir.path().join("nabungo-export-2026-10-20-2"));
        let third = k.ledger.export_csv(dir.path(), TODAY).unwrap();
        assert_eq!(third, dir.path().join("nabungo-export-2026-10-20-3"));
        assert_eq!(
            std::fs::read(first.join("transactions.csv")).unwrap(),
            before
        );
        let mut files: Vec<_> = std::fs::read_dir(&first)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        assert_eq!(
            files,
            [
                "accounts.csv",
                "budgets.csv",
                "categories.csv",
                "transactions.csv"
            ]
        );
    }

    #[test]
    fn csv_has_bom_headers_and_one_row_per_posting() {
        let mut k = seeded();
        let (makan, transport) = (k.cat("Makan & Minum"), k.cat("Transportasi"));
        let account_id = k.bca;
        k.ledger
            .create_txn(&TxnForm {
                body: FormBody::Expense {
                    account_id,
                    amount: 30_000,
                    splits: vec![
                        SplitLine {
                            category_id: makan,
                            amount: 20_000,
                            memo: None,
                        },
                        SplitLine {
                            category_id: transport,
                            amount: 10_000,
                            memo: None,
                        },
                    ],
                },
                date: date!(2026 - 10 - 06),
                payee: None,
                note: Some("split".into()),
                tags: vec![],
            })
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let out = k.ledger.export_csv(dir.path(), TODAY).unwrap();
        let headers = [
            (
                "transactions.csv",
                "txn_id,date,kind,status,payee,note,tags,account,category,amount,currency,memo",
            ),
            (
                "accounts.csv",
                "id,name,kind,group,currency,archived,include_in_net_worth,balance",
            ),
            ("categories.csv", "id,name,kind,parent,archived"),
            ("budgets.csv", "period,category,amount"),
        ];
        for (file, header) in headers {
            let text = read(&out.join(file));
            assert_eq!(text.split("\r\n").next().unwrap(), header, "{file}");
        }
        let rows = records(&out.join("transactions.csv"));
        let split: Vec<_> = rows.iter().filter(|r| &r[5] == "split").collect();
        // Satu posting per split; tiap posting membawa akun dan kategori.
        assert_eq!(split.len(), 2);
        let mut cats: Vec<_> = split.iter().map(|r| r[8].to_owned()).collect();
        cats.sort();
        assert_eq!(cats, ["Makan & Minum", "Transportasi"]);
        assert!(split.iter().all(|r| &r[7] == "BCA"));

        let accounts = records(&out.join("accounts.csv"));
        let bca = accounts.iter().find(|r| &r[1] == "BCA").unwrap();
        assert_eq!(bca[7], k.ledger.balance(k.bca).unwrap().to_string());
        let budgets = records(&out.join("budgets.csv"));
        assert_eq!(budgets.len(), 1);
        assert_eq!(&budgets[0][1], "Makan & Minum");
    }

    #[test]
    fn csv_escapes_and_neutralizes_formulas() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let makan = k.cat("Makan & Minum");
        let account_id = k.bca;
        k.ledger
            .create_txn(&TxnForm {
                body: FormBody::Expense {
                    account_id,
                    amount: 15_000,
                    splits: vec![SplitLine {
                        category_id: makan,
                        amount: 15_000,
                        memo: Some("-memo".into()),
                    }],
                },
                date: date!(2026 - 10 - 02),
                payee: Some("=HYPERLINK(\"x\")".into()),
                note: Some("a, \"b\"\nc".into()),
                tags: vec!["@kantor".into(), "rutin".into()],
            })
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let out = k.ledger.export_csv(dir.path(), TODAY).unwrap();
        let raw = read(&out.join("transactions.csv"));
        assert!(raw.contains("\"'=HYPERLINK(\"\"x\"\")\""), "{raw}");
        assert!(raw.contains("\"a, \"\"b\"\"\nc\""), "{raw}");

        let rows = records(&out.join("transactions.csv"));
        // opening + expense, masing-masing satu posting.
        assert_eq!(rows.len(), 2);
        let exp = rows.iter().find(|r| &r[2] == "expense").unwrap();
        assert_eq!(&exp[4], "'=HYPERLINK(\"x\")");
        assert_eq!(&exp[5], "a, \"b\"\nc");
        assert_eq!(&exp[6], "'@kantor;rutin");
        assert_eq!(&exp[8], "Makan & Minum");
        // Kolom angka tidak dinetralisasi.
        assert_eq!(&exp[9], "-15000");
        assert_eq!(&exp[11], "'-memo");
    }

    #[test]
    fn child_category_written_with_parent() {
        let mut k = Kit::new(1, date!(2026 - 10 - 01));
        let makan = k.cat("Makan & Minum");
        k.ledger
            .create_category(
                CategoryKind::Expense,
                &CategoryFields {
                    parent_id: Some(makan),
                    ..CategoryFields::named("Kafe baru")
                },
            )
            .unwrap();
        k.expense(date!(2026 - 10 - 02), "Kafe baru", 50_000);
        let dir = tempfile::tempdir().unwrap();
        let out = k.ledger.export_csv(dir.path(), TODAY).unwrap();
        let cats = records(&out.join("categories.csv"));
        let child = cats.iter().find(|r| &r[1] == "Kafe baru").unwrap();
        assert_eq!(&child[3], "Makan & Minum");
        let rows = records(&out.join("transactions.csv"));
        assert!(rows.iter().any(|r| &r[8] == "Makan & Minum / Kafe baru"));
    }
}
