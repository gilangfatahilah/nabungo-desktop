# Fase 1 — Fondasi Domain & Storage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mengunci aturan uang Nabungo (Money, periode budget, invariant transaksi, saldo dari postings) dengan test sebelum ada UI.

**Architecture:** `nabungo-core` berisi tipe & validasi murni tanpa IO. `nabungo-storage` menulis/membaca tabel `account`, `category`, `txn`, `posting` lewat rusqlite. `nabungo-app` menyediakan `Ledger` yang me-resolve input berbasis ID, memvalidasi lewat `core`, lalu menulis lewat `storage` dalam satu SQLite transaction.

**Tech Stack:** Rust 2024, `time` 0.3, `uuid` (v7), `rusqlite` 0.40 (bundled), `rusqlite_migration` 2.x, `thiserror` 2, `proptest`.

**Spec:** `PRD.md` §4.2, §5 (acceptance criteria), §6 (skema + invariant), §7. Roadmap: `docs/plans/2026-10-12-v1-roadmap.md`.

## Global Constraints

- Nominal uang selalu `i64` satuan minor; IDR `decimals = 0`. Tidak ada `f32`/`f64` di `nabungo-core` (`#![deny(clippy::float_arithmetic)]`).
- Tanggal transaksi `TEXT` format `YYYY-MM-DD`; timestamp audit ISO-8601 UTC (RFC 3339).
- ID: UUIDv7 disimpan sebagai TEXT (format hyphenated).
- `nabungo-core` tidak boleh bergantung pada rusqlite, tauri, atau filesystem.
- Invariant transaksi ditegakkan di `core::txn::validate`, bukan trigger SQL.
- Semua perubahan satu transaksi (header + postings) dalam satu `rusqlite::Transaction`.
- Saldo = Σ `posting.amount` dari `txn` dengan `deleted_at IS NULL AND status = 'posted'`. Saldo awal adalah posting `opening`, bukan kolom.
- Migrasi yang sudah ada (`0001_init.sql`) tidak diubah.
- Setiap task diakhiri `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` hijau.

## Review Focus

1. Saldo awal 0 → tidak boleh membuat transaksi `opening` kosong (posting 0 akan ditolak validasi). Test di Task 6.
2. Transfer ke akun yang sama → ditolak `TransferSameAccount`. Test di Task 4.
3. Refund = expense bernilai positif (uang masuk) dengan kategori expense → diterima. Test di Task 4.
4. Transaksi yang di-soft-delete tidak dihitung di saldo; undo mengembalikannya; delete ID tak dikenal → `NotFound`, bukan sukses diam-diam. Test di Task 5 dan Task 6.
5. Jumlah dua leg transfer overflow `i64` → error `Overflow`, bukan panic. Test di Task 4.
6. Tanggal awal periode 29–31 ditolak (bulan pendek tidak punya tanggal itu). Test di Task 2.

---

### Task 1: `Money` dan `CurrencyCode`

**Files:**
- Create: `crates/core/src/money.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Produces: `CurrencyCode` (`Copy`, `CurrencyCode::IDR`, `parse(&str) -> Result<_, MoneyError>`, `as_str()`, `Display`, serde sebagai string); `Money { amount: i64, currency: CurrencyCode }` dengan `new`, `idr`, `checked_add`, `checked_neg`, `is_zero`; `MoneyError { InvalidCurrency(String), CurrencyMismatch(CurrencyCode, CurrencyCode), Overflow }`.

- [ ] **Step 1: Tulis test yang gagal** — buat `crates/core/src/money.rs` hanya berisi modul test, dan tambahkan `pub mod money;` di `lib.rs`.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_uppercase_iso_code() {
        assert_eq!(CurrencyCode::parse("IDR").unwrap(), CurrencyCode::IDR);
        assert_eq!(CurrencyCode::IDR.as_str(), "IDR");
    }

    #[test]
    fn parse_rejects_invalid_codes() {
        for bad in ["idr", "ID", "IDRX", "I1R", ""] {
            assert!(CurrencyCode::parse(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn add_same_currency() {
        let sum = Money::idr(25_000).checked_add(Money::idr(12_500)).unwrap();
        assert_eq!(sum, Money::idr(37_500));
    }

    #[test]
    fn add_different_currency_fails() {
        let usd = CurrencyCode::parse("USD").unwrap();
        let err = Money::idr(1).checked_add(Money::new(1, usd)).unwrap_err();
        assert_eq!(err, MoneyError::CurrencyMismatch(CurrencyCode::IDR, usd));
    }

    #[test]
    fn add_overflow_fails() {
        let err = Money::idr(i64::MAX).checked_add(Money::idr(1)).unwrap_err();
        assert_eq!(err, MoneyError::Overflow);
    }

    #[test]
    fn neg_min_overflows() {
        assert_eq!(Money::idr(i64::MIN).checked_neg().unwrap_err(), MoneyError::Overflow);
        assert_eq!(Money::idr(5).checked_neg().unwrap(), Money::idr(-5));
    }

    #[test]
    fn currency_serializes_as_string() {
        let json = serde_json::to_string(&Money::idr(10)).unwrap();
        assert_eq!(json, r#"{"amount":10,"currency":"IDR"}"#);
    }
}
```

Tambahkan dev-dependency: `cargo add --dev serde_json -p nabungo-core`.

- [ ] **Step 2: Jalankan, pastikan gagal**

Run: `cargo test -p nabungo-core money::`
Expected: FAIL compile — `cannot find type CurrencyCode`.

- [ ] **Step 3: Implementasi** — tambahkan di atas modul test:

```rust
//! Uang sebagai integer satuan minor. Tidak pernah floating point.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    #[error("invalid currency code: {0:?}")]
    InvalidCurrency(String),
    #[error("currency mismatch: {0} vs {1}")]
    CurrencyMismatch(CurrencyCode, CurrencyCode),
    #[error("amount overflow")]
    Overflow,
}

/// Kode ISO 4217 tiga huruf kapital.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CurrencyCode([u8; 3]);

impl CurrencyCode {
    pub const IDR: CurrencyCode = CurrencyCode(*b"IDR");

    pub fn parse(s: &str) -> Result<Self, MoneyError> {
        match s.as_bytes() {
            &[a, b, c] if [a, b, c].iter().all(u8::is_ascii_uppercase) => Ok(Self([a, b, c])),
            _ => Err(MoneyError::InvalidCurrency(s.to_owned())),
        }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("currency code is ASCII")
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<String> for CurrencyCode {
    type Error = MoneyError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl From<CurrencyCode> for String {
    fn from(c: CurrencyCode) -> Self {
        c.as_str().to_owned()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    pub amount: i64,
    pub currency: CurrencyCode,
}

impl Money {
    pub const fn new(amount: i64, currency: CurrencyCode) -> Self {
        Self { amount, currency }
    }

    pub const fn idr(amount: i64) -> Self {
        Self::new(amount, CurrencyCode::IDR)
    }

    pub fn checked_add(self, other: Money) -> Result<Money, MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch(self.currency, other.currency));
        }
        let amount = self.amount.checked_add(other.amount).ok_or(MoneyError::Overflow)?;
        Ok(Money::new(amount, self.currency))
    }

    pub fn checked_neg(self) -> Result<Money, MoneyError> {
        let amount = self.amount.checked_neg().ok_or(MoneyError::Overflow)?;
        Ok(Money::new(amount, self.currency))
    }

    pub fn is_zero(&self) -> bool {
        self.amount == 0
    }
}
```

Di `lib.rs` tambahkan re-export: `pub use money::{CurrencyCode, Money, MoneyError};`

- [ ] **Step 4: Jalankan, pastikan lulus**

Run: `cargo test -p nabungo-core money::`
Expected: 7 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "feat(core): add Money and CurrencyCode"
```

---

### Task 2: Tanggal dan periode budget (`period_of`)

**Files:**
- Create: `crates/core/src/date.rs`, `crates/core/src/period.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml`

**Interfaces:**
- Produces: `date::parse_date(&str) -> Result<time::Date, time::error::Parse>`, `date::format_date(time::Date) -> String`; `PeriodStartDay::new(u8) -> Result<Self, PeriodError>`, `PeriodStartDay::FIRST`, `.day() -> u8`; `BudgetPeriod { year: i32, month: u8 }` dengan `new`, `next`, `prev`, `key() -> String` (`"2026-11"`), `date_range(PeriodStartDay) -> (Date, Date)` (setengah terbuka `[start, end)`); `period_of(Date, PeriodStartDay) -> BudgetPeriod`; `PeriodError { InvalidStartDay(u8), InvalidMonth(u8) }`.

- [ ] **Step 1: Tambah dependensi**

Run: `cargo add time -p nabungo-core --features macros,parsing,formatting`

- [ ] **Step 2: Tulis test yang gagal** — `crates/core/src/date.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn roundtrip() {
        let d = parse_date("2026-10-12").unwrap();
        assert_eq!(d, date!(2026 - 10 - 12));
        assert_eq!(format_date(d), "2026-10-12");
    }

    #[test]
    fn rejects_other_formats() {
        for bad in ["2026-13-01", "12/10/2026", "2026-1-5", "2026-02-30", ""] {
            assert!(parse_date(bad).is_err(), "{bad} should be rejected");
        }
    }
}
```

`crates/core/src/period.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use time::macros::date;

    fn p(year: i32, month: u8) -> BudgetPeriod {
        BudgetPeriod::new(year, month).unwrap()
    }

    #[test]
    fn calendar_month_when_start_day_is_first() {
        let s = PeriodStartDay::FIRST;
        assert_eq!(period_of(date!(2026 - 10 - 01), s), p(2026, 10));
        assert_eq!(period_of(date!(2026 - 10 - 31), s), p(2026, 10));
    }

    #[test]
    fn payday_start_day_25() {
        // PRD §5: awal bulan budget = 25 → transaksi 26 Oktober masuk periode November.
        let s = PeriodStartDay::new(25).unwrap();
        assert_eq!(period_of(date!(2026 - 10 - 24), s), p(2026, 10));
        assert_eq!(period_of(date!(2026 - 10 - 25), s), p(2026, 11));
        assert_eq!(period_of(date!(2026 - 10 - 26), s), p(2026, 11));
        assert_eq!(period_of(date!(2026 - 12 - 25), s), p(2027, 1));
    }

    #[test]
    fn date_range_of_payday_period() {
        let s = PeriodStartDay::new(25).unwrap();
        assert_eq!(p(2026, 11).date_range(s), (date!(2026 - 10 - 25), date!(2026 - 11 - 25)));
        assert_eq!(p(2027, 1).date_range(s), (date!(2026 - 12 - 25), date!(2027 - 01 - 25)));
        assert_eq!(
            p(2026, 12).date_range(PeriodStartDay::FIRST),
            (date!(2026 - 12 - 01), date!(2027 - 01 - 01))
        );
    }

    #[test]
    fn start_day_must_be_1_to_28() {
        for bad in [0, 29, 30, 31] {
            assert_eq!(PeriodStartDay::new(bad), Err(PeriodError::InvalidStartDay(bad)));
        }
        assert!(PeriodStartDay::new(28).is_ok());
    }

    #[test]
    fn key_is_zero_padded() {
        assert_eq!(p(2026, 3).key(), "2026-03");
        assert!(BudgetPeriod::new(2026, 13).is_err());
    }

    proptest! {
        #[test]
        fn date_falls_inside_its_period_range(jd in 2_451_545i32..2_488_070, start in 1u8..=28) {
            let d = time::Date::from_julian_day(jd).unwrap();
            let s = PeriodStartDay::new(start).unwrap();
            let (from, to) = period_of(d, s).date_range(s);
            prop_assert!(from <= d && d < to, "{d} not in [{from}, {to})");
        }
    }
}
```

Di `lib.rs` tambahkan `pub mod date;` dan `pub mod period;`.

- [ ] **Step 3: Jalankan, pastikan gagal**

Run: `cargo test -p nabungo-core date:: period::`
Expected: FAIL compile — `cannot find function parse_date`, `cannot find type BudgetPeriod`.

- [ ] **Step 4: Implementasi `date.rs`** (di atas modul test):

```rust
//! Tanggal transaksi: `YYYY-MM-DD`, tanpa jam dan tanpa timezone.

use time::Date;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;

const FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[year]-[month]-[day]");

pub fn parse_date(s: &str) -> Result<Date, time::error::Parse> {
    Date::parse(s, FORMAT)
}

pub fn format_date(d: Date) -> String {
    d.format(FORMAT).expect("YYYY-MM-DD is always formattable")
}
```

- [ ] **Step 5: Implementasi `period.rs`** (di atas modul test):

```rust
//! Periode budget. Satu-satunya tempat aturan "tanggal gajian" didefinisikan (PRD §10).

use time::{Date, Month};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PeriodError {
    #[error("period start day must be 1..=28, got {0}")]
    InvalidStartDay(u8),
    #[error("month must be 1..=12, got {0}")]
    InvalidMonth(u8),
}

/// Tanggal awal periode budget. Dibatasi 1..=28 agar ada di setiap bulan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodStartDay(u8);

impl PeriodStartDay {
    pub const FIRST: PeriodStartDay = PeriodStartDay(1);

    pub fn new(day: u8) -> Result<Self, PeriodError> {
        if (1..=28).contains(&day) {
            Ok(Self(day))
        } else {
            Err(PeriodError::InvalidStartDay(day))
        }
    }

    pub fn day(self) -> u8 {
        self.0
    }
}

/// Periode budget dinamai menurut bulan tempat periode itu **berakhir**:
/// dengan start day 25, periode "2026-11" = 25 Okt s/d 24 Nov.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BudgetPeriod {
    pub year: i32,
    pub month: u8,
}

impl BudgetPeriod {
    pub fn new(year: i32, month: u8) -> Result<Self, PeriodError> {
        if (1..=12).contains(&month) {
            Ok(Self { year, month })
        } else {
            Err(PeriodError::InvalidMonth(month))
        }
    }

    pub fn next(self) -> Self {
        if self.month == 12 {
            Self { year: self.year + 1, month: 1 }
        } else {
            Self { year: self.year, month: self.month + 1 }
        }
    }

    pub fn prev(self) -> Self {
        if self.month == 1 {
            Self { year: self.year - 1, month: 12 }
        } else {
            Self { year: self.year, month: self.month - 1 }
        }
    }

    /// Kunci yang disimpan di kolom `budget.period`, mis. `"2026-11"`.
    pub fn key(&self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }

    /// Rentang tanggal setengah terbuka `[start, end)`.
    pub fn date_range(self, start: PeriodStartDay) -> (Date, Date) {
        if start.0 == 1 {
            let next = self.next();
            (ymd(self.year, self.month, 1), ymd(next.year, next.month, 1))
        } else {
            let prev = self.prev();
            (ymd(prev.year, prev.month, start.0), ymd(self.year, self.month, start.0))
        }
    }
}

pub fn period_of(date: Date, start: PeriodStartDay) -> BudgetPeriod {
    let current = BudgetPeriod { year: date.year(), month: u8::from(date.month()) };
    if start.0 == 1 || date.day() < start.0 {
        current
    } else {
        current.next()
    }
}

fn ymd(year: i32, month: u8, day: u8) -> Date {
    let month = Month::try_from(month).expect("month validated by BudgetPeriod");
    Date::from_calendar_date(year, month, day).expect("day <= 28 exists in every month")
}
```

Re-export di `lib.rs`: `pub use period::{BudgetPeriod, PeriodError, PeriodStartDay, period_of};`

- [ ] **Step 6: Jalankan, pastikan lulus**

Run: `cargo test -p nabungo-core date:: period::`
Expected: 8 passed (2 date + 6 period).

- [ ] **Step 7: Commit**

```bash
git add crates/core
git commit -m "feat(core): add date helpers and budget period_of"
```

---

### Task 3: Enum jenis akun, kategori, transaksi

**Files:**
- Create: `crates/core/src/kinds.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Produces: `AccountKind { Cash, Bank, Ewallet, Investment, Receivable, CreditCard, Loan }` dengan `ALL`, `as_str()`, `parse(&str) -> Option<Self>`, `is_liability()`; `CategoryKind { Income, Expense }` dengan `as_str`/`parse`; `TxnKind { Income, Expense, Transfer, Adjustment, Valuation, Opening }` dengan `ALL`, `as_str`/`parse`. String `as_str()` **identik** dengan nilai `CHECK` di `0001_init.sql`.

- [ ] **Step 1: Tulis test yang gagal** — `crates/core/src/kinds.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_kind_roundtrip_matches_schema() {
        let names: Vec<_> = AccountKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            names,
            ["cash", "bank", "ewallet", "investment", "receivable", "credit_card", "loan"]
        );
        for k in AccountKind::ALL {
            assert_eq!(AccountKind::parse(k.as_str()), Some(k));
        }
        assert_eq!(AccountKind::parse("savings"), None);
    }

    #[test]
    fn only_credit_card_and_loan_are_liabilities() {
        let liabilities: Vec<_> = AccountKind::ALL.into_iter().filter(|k| k.is_liability()).collect();
        assert_eq!(liabilities, [AccountKind::CreditCard, AccountKind::Loan]);
    }

    #[test]
    fn txn_kind_roundtrip_matches_schema() {
        let names: Vec<_> = TxnKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(names, ["income", "expense", "transfer", "adjustment", "valuation", "opening"]);
        for k in TxnKind::ALL {
            assert_eq!(TxnKind::parse(k.as_str()), Some(k));
        }
    }

    #[test]
    fn category_kind_roundtrip() {
        assert_eq!(CategoryKind::parse("income"), Some(CategoryKind::Income));
        assert_eq!(CategoryKind::parse("expense"), Some(CategoryKind::Expense));
        assert_eq!(CategoryKind::Expense.as_str(), "expense");
        assert_eq!(CategoryKind::parse("transfer"), None);
    }
}
```

Tambahkan `pub mod kinds;` di `lib.rs`.

- [ ] **Step 2: Jalankan, pastikan gagal**

Run: `cargo test -p nabungo-core kinds::`
Expected: FAIL compile — `cannot find type AccountKind`.

- [ ] **Step 3: Implementasi** (di atas modul test):

```rust
//! Jenis akun, kategori, dan transaksi. String-nya harus sama dengan CHECK di skema SQL.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Cash,
    Bank,
    Ewallet,
    Investment,
    Receivable,
    CreditCard,
    Loan,
}

impl AccountKind {
    pub const ALL: [AccountKind; 7] = [
        Self::Cash,
        Self::Bank,
        Self::Ewallet,
        Self::Investment,
        Self::Receivable,
        Self::CreditCard,
        Self::Loan,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cash => "cash",
            Self::Bank => "bank",
            Self::Ewallet => "ewallet",
            Self::Investment => "investment",
            Self::Receivable => "receivable",
            Self::CreditCard => "credit_card",
            Self::Loan => "loan",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }

    pub fn is_liability(self) -> bool {
        matches!(self, Self::CreditCard | Self::Loan)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryKind {
    Income,
    Expense,
}

impl CategoryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [Self::Income, Self::Expense].into_iter().find(|k| k.as_str() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TxnKind {
    Income,
    Expense,
    Transfer,
    Adjustment,
    Valuation,
    Opening,
}

impl TxnKind {
    pub const ALL: [TxnKind; 6] = [
        Self::Income,
        Self::Expense,
        Self::Transfer,
        Self::Adjustment,
        Self::Valuation,
        Self::Opening,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
            Self::Transfer => "transfer",
            Self::Adjustment => "adjustment",
            Self::Valuation => "valuation",
            Self::Opening => "opening",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }
}
```

Re-export di `lib.rs`: `pub use kinds::{AccountKind, CategoryKind, TxnKind};`

- [ ] **Step 4: Jalankan, pastikan lulus**

Run: `cargo test -p nabungo-core kinds::`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "feat(core): add account, category and txn kinds"
```

---

### Task 4: `TxnDraft` dan validasi invariant

**Files:**
- Create: `crates/core/src/txn.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml`

**Interfaces:**
- Consumes: `AccountKind`, `CategoryKind`, `TxnKind` (Task 3), `CurrencyCode` (Task 1).
- Produces:
  - `AccountRef { id: Uuid, kind: AccountKind, currency: CurrencyCode }` (`Copy`)
  - `CategoryRef { id: Uuid, kind: CategoryKind }` (`Copy`)
  - `PostingDraft { account: AccountRef, category: Option<CategoryRef>, amount: i64, memo: Option<String> }`
  - `TxnDraft { kind: TxnKind, date: time::Date, note: Option<String>, postings: Vec<PostingDraft> }`
  - `validate(&TxnDraft) -> Result<(), TxnError>`
  - `TxnError { NoPostings, ZeroAmount, MissingCategory, WrongCategoryKind { expected: CategoryKind }, UnexpectedCategory, TransferLegCount(usize), TransferSameAccount, CurrencyMismatch, Unbalanced(i64), Overflow, ValuationNonInvestment, OpeningPostingCount(usize) }`

Aturan (PRD §4.2 & §6):
- Semua transaksi: ≥ 1 posting, tidak ada posting bernilai 0.
- `income` / `expense`: setiap posting wajib punya kategori dengan kind yang sama. Refund = expense dengan amount positif.
- `transfer`: tepat 2 posting tanpa kategori (leg), akun berbeda, currency sama (v1 IDR saja), jumlah = 0 (checked). Posting lain = biaya admin, wajib kategori expense.
- `adjustment`: tanpa kategori.
- `valuation`: tanpa kategori, semua posting ke akun `investment`.
- `opening`: tepat 1 posting, tanpa kategori.

- [ ] **Step 1: Tambah dependensi**

Run: `cargo add uuid -p nabungo-core --features serde,v7`

(`v7` hanya dipakai di test untuk membuat ID; produksi membuat ID di `storage`.)

- [ ] **Step 2: Tulis test yang gagal** — `crates/core/src/txn.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use time::macros::date;

    fn account(kind: AccountKind) -> AccountRef {
        AccountRef { id: Uuid::now_v7(), kind, currency: CurrencyCode::IDR }
    }

    fn category(kind: CategoryKind) -> CategoryRef {
        CategoryRef { id: Uuid::now_v7(), kind }
    }

    fn posting(account: AccountRef, category: Option<CategoryRef>, amount: i64) -> PostingDraft {
        PostingDraft { account, category, amount, memo: None }
    }

    fn draft(kind: TxnKind, postings: Vec<PostingDraft>) -> TxnDraft {
        TxnDraft { kind, date: date!(2026 - 10 - 12), note: None, postings }
    }

    fn transfer(a: AccountRef, b: AccountRef, out: i64, inc: i64) -> TxnDraft {
        draft(TxnKind::Transfer, vec![posting(a, None, out), posting(b, None, inc)])
    }

    #[test]
    fn expense_with_expense_category_is_valid() {
        let d = draft(
            TxnKind::Expense,
            vec![posting(account(AccountKind::Ewallet), Some(category(CategoryKind::Expense)), -25_000)],
        );
        assert_eq!(validate(&d), Ok(()));
    }

    #[test]
    fn refund_is_positive_expense() {
        let d = draft(
            TxnKind::Expense,
            vec![posting(account(AccountKind::Bank), Some(category(CategoryKind::Expense)), 15_000)],
        );
        assert_eq!(validate(&d), Ok(()));
    }

    #[test]
    fn split_expense_needs_category_on_every_posting() {
        let acc = account(AccountKind::Bank);
        let d = draft(
            TxnKind::Expense,
            vec![
                posting(acc, Some(category(CategoryKind::Expense)), -10_000),
                posting(acc, None, -5_000),
            ],
        );
        assert_eq!(validate(&d), Err(TxnError::MissingCategory));
    }

    #[test]
    fn income_rejects_expense_category() {
        let d = draft(
            TxnKind::Income,
            vec![posting(account(AccountKind::Bank), Some(category(CategoryKind::Expense)), 1_000)],
        );
        assert_eq!(
            validate(&d),
            Err(TxnError::WrongCategoryKind { expected: CategoryKind::Income })
        );
    }

    #[test]
    fn empty_and_zero_postings_rejected() {
        assert_eq!(validate(&draft(TxnKind::Adjustment, vec![])), Err(TxnError::NoPostings));
        let zero = draft(TxnKind::Adjustment, vec![posting(account(AccountKind::Bank), None, 0)]);
        assert_eq!(validate(&zero), Err(TxnError::ZeroAmount));
    }

    #[test]
    fn balanced_transfer_is_valid() {
        let d = transfer(account(AccountKind::Bank), account(AccountKind::Ewallet), -100_000, 100_000);
        assert_eq!(validate(&d), Ok(()));
    }

    #[test]
    fn transfer_with_admin_fee_is_valid() {
        let bank = account(AccountKind::Bank);
        let mut d = transfer(bank, account(AccountKind::Ewallet), -100_000, 100_000);
        d.postings.push(posting(bank, Some(category(CategoryKind::Expense)), -1_000));
        assert_eq!(validate(&d), Ok(()));
    }

    #[test]
    fn transfer_fee_must_be_expense_category() {
        let bank = account(AccountKind::Bank);
        let mut d = transfer(bank, account(AccountKind::Ewallet), -100_000, 100_000);
        d.postings.push(posting(bank, Some(category(CategoryKind::Income)), -1_000));
        assert_eq!(
            validate(&d),
            Err(TxnError::WrongCategoryKind { expected: CategoryKind::Expense })
        );
    }

    #[test]
    fn unbalanced_transfer_rejected() {
        let d = transfer(account(AccountKind::Bank), account(AccountKind::Ewallet), -100_000, 90_000);
        assert_eq!(validate(&d), Err(TxnError::Unbalanced(-10_000)));
    }

    #[test]
    fn transfer_to_same_account_rejected() {
        let bank = account(AccountKind::Bank);
        assert_eq!(validate(&transfer(bank, bank, -1, 1)), Err(TxnError::TransferSameAccount));
    }

    #[test]
    fn transfer_needs_exactly_two_legs() {
        let d = draft(TxnKind::Transfer, vec![posting(account(AccountKind::Bank), None, -1)]);
        assert_eq!(validate(&d), Err(TxnError::TransferLegCount(1)));
    }

    #[test]
    fn transfer_across_currencies_rejected_in_v1() {
        let usd = CurrencyCode::parse("USD").unwrap();
        let foreign = AccountRef { currency: usd, ..account(AccountKind::Bank) };
        let d = transfer(account(AccountKind::Bank), foreign, -1, 1);
        assert_eq!(validate(&d), Err(TxnError::CurrencyMismatch));
    }

    #[test]
    fn transfer_sum_overflow_is_error_not_panic() {
        let d = transfer(account(AccountKind::Bank), account(AccountKind::Ewallet), i64::MAX, 1);
        assert_eq!(validate(&d), Err(TxnError::Overflow));
    }

    #[test]
    fn valuation_only_for_investment() {
        let ok = draft(TxnKind::Valuation, vec![posting(account(AccountKind::Investment), None, 50_000)]);
        assert_eq!(validate(&ok), Ok(()));
        let bad = draft(TxnKind::Valuation, vec![posting(account(AccountKind::Bank), None, 50_000)]);
        assert_eq!(validate(&bad), Err(TxnError::ValuationNonInvestment));
    }

    #[test]
    fn adjustment_and_opening_have_no_category() {
        let cat = Some(category(CategoryKind::Expense));
        let adj = draft(TxnKind::Adjustment, vec![posting(account(AccountKind::Bank), cat, -1)]);
        assert_eq!(validate(&adj), Err(TxnError::UnexpectedCategory));
        let acc = account(AccountKind::Bank);
        let two = draft(TxnKind::Opening, vec![posting(acc, None, 1), posting(acc, None, 2)]);
        assert_eq!(validate(&two), Err(TxnError::OpeningPostingCount(2)));
    }

    proptest! {
        #[test]
        fn any_balanced_transfer_is_valid(x in 1i64..=i64::MAX) {
            let d = transfer(account(AccountKind::Bank), account(AccountKind::Ewallet), -x, x);
            prop_assert_eq!(validate(&d), Ok(()));
        }

        #[test]
        fn any_unbalanced_transfer_is_rejected(x in 1i64..=1_000_000_000_000, y in 1i64..=1_000_000_000_000) {
            prop_assume!(x != y);
            let d = transfer(account(AccountKind::Bank), account(AccountKind::Ewallet), -x, y);
            prop_assert_eq!(validate(&d), Err(TxnError::Unbalanced(y - x)));
        }
    }
}
```

Tambahkan `pub mod txn;` di `lib.rs`.

- [ ] **Step 3: Jalankan, pastikan gagal**

Run: `cargo test -p nabungo-core txn::`
Expected: FAIL compile — `cannot find function validate`.

- [ ] **Step 4: Implementasi** (di atas modul test):

```rust
//! Draft transaksi (header + postings) dan invariant PRD §6 no. 1–3.

use time::Date;
use uuid::Uuid;

use crate::{AccountKind, CategoryKind, CurrencyCode, TxnKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountRef {
    pub id: Uuid,
    pub kind: AccountKind,
    pub currency: CurrencyCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryRef {
    pub id: Uuid,
    pub kind: CategoryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingDraft {
    pub account: AccountRef,
    pub category: Option<CategoryRef>,
    /// +masuk / -keluar, satuan minor currency akun.
    pub amount: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnDraft {
    pub kind: TxnKind,
    pub date: Date,
    pub note: Option<String>,
    pub postings: Vec<PostingDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TxnError {
    #[error("transaction has no postings")]
    NoPostings,
    #[error("posting amount must not be zero")]
    ZeroAmount,
    #[error("posting requires a category")]
    MissingCategory,
    #[error("posting category must be {expected:?}")]
    WrongCategoryKind { expected: CategoryKind },
    #[error("posting must not have a category")]
    UnexpectedCategory,
    #[error("transfer needs exactly 2 uncategorized postings, got {0}")]
    TransferLegCount(usize),
    #[error("transfer source and destination must differ")]
    TransferSameAccount,
    #[error("transfer between different currencies is not supported")]
    CurrencyMismatch,
    #[error("transfer is unbalanced by {0}")]
    Unbalanced(i64),
    #[error("amount overflow")]
    Overflow,
    #[error("valuation is only allowed on investment accounts")]
    ValuationNonInvestment,
    #[error("opening balance needs exactly 1 posting, got {0}")]
    OpeningPostingCount(usize),
}

pub fn validate(draft: &TxnDraft) -> Result<(), TxnError> {
    let postings = &draft.postings;
    if postings.is_empty() {
        return Err(TxnError::NoPostings);
    }
    if postings.iter().any(|p| p.amount == 0) {
        return Err(TxnError::ZeroAmount);
    }
    match draft.kind {
        TxnKind::Income => require_category(postings, CategoryKind::Income),
        TxnKind::Expense => require_category(postings, CategoryKind::Expense),
        TxnKind::Transfer => validate_transfer(postings),
        TxnKind::Adjustment => forbid_category(postings),
        TxnKind::Valuation => {
            forbid_category(postings)?;
            if postings.iter().all(|p| p.account.kind == AccountKind::Investment) {
                Ok(())
            } else {
                Err(TxnError::ValuationNonInvestment)
            }
        }
        TxnKind::Opening => {
            if postings.len() != 1 {
                return Err(TxnError::OpeningPostingCount(postings.len()));
            }
            forbid_category(postings)
        }
    }
}

fn require_category<'a>(
    postings: impl IntoIterator<Item = &'a PostingDraft>,
    expected: CategoryKind,
) -> Result<(), TxnError> {
    for p in postings {
        match p.category {
            None => return Err(TxnError::MissingCategory),
            Some(c) if c.kind != expected => return Err(TxnError::WrongCategoryKind { expected }),
            Some(_) => {}
        }
    }
    Ok(())
}

fn forbid_category(postings: &[PostingDraft]) -> Result<(), TxnError> {
    if postings.iter().any(|p| p.category.is_some()) {
        Err(TxnError::UnexpectedCategory)
    } else {
        Ok(())
    }
}

fn validate_transfer(postings: &[PostingDraft]) -> Result<(), TxnError> {
    let (legs, fees): (Vec<&PostingDraft>, Vec<&PostingDraft>) =
        postings.iter().partition(|p| p.category.is_none());
    let [from, to] = legs[..] else {
        return Err(TxnError::TransferLegCount(legs.len()));
    };
    if from.account.id == to.account.id {
        return Err(TxnError::TransferSameAccount);
    }
    if from.account.currency != to.account.currency {
        return Err(TxnError::CurrencyMismatch);
    }
    let sum = from.amount.checked_add(to.amount).ok_or(TxnError::Overflow)?;
    if sum != 0 {
        return Err(TxnError::Unbalanced(sum));
    }
    require_category(fees, CategoryKind::Expense)
}
```

Re-export di `lib.rs`: `pub use txn::{AccountRef, CategoryRef, PostingDraft, TxnDraft, TxnError, validate};`

- [ ] **Step 5: Jalankan, pastikan lulus**

Run: `cargo test -p nabungo-core txn::`
Expected: 17 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/core
git commit -m "feat(core): validate transaction invariants"
```

---

### Task 5: Repository akun, kategori, transaksi + query saldo

**Files:**
- Create: `crates/storage/src/account.rs`, `crates/storage/src/category.rs`, `crates/storage/src/txn.rs`
- Modify: `crates/storage/src/lib.rs`, `crates/storage/Cargo.toml`

**Interfaces:**
- Consumes: `AccountKind`, `CategoryKind`, `CurrencyCode`, `AccountRef`, `CategoryRef`, `TxnDraft`, `date::format_date` dari `nabungo-core`.
- Produces:
  - `nabungo_storage::rusqlite` (re-export, agar `app` memakai versi yang sama)
  - `StorageError::Corrupt(String)` (varian baru)
  - `now_utc() -> String` (RFC 3339)
  - `account::NewAccount { name: String, kind: AccountKind, currency: CurrencyCode }`
  - `account::insert_account(&Transaction, &NewAccount, now: &str) -> Result<Uuid>`
  - `account::get_account_ref(&Connection, Uuid) -> Result<Option<AccountRef>>`
  - `category::insert_category(&Transaction, CategoryKind, name: &str, parent: Option<Uuid>) -> Result<Uuid>`
  - `category::get_category_ref(&Connection, Uuid) -> Result<Option<CategoryRef>>`
  - `txn::insert_txn(&Transaction, &TxnDraft, now: &str) -> Result<Uuid>` (tidak memvalidasi — pemanggil wajib `core::validate` dulu)
  - `txn::set_deleted(&Transaction, Uuid, deleted_at: Option<&str>, now: &str) -> Result<bool>` (`true` bila baris ada)
  - `txn::account_balance(&Connection, Uuid) -> Result<i64>`

`&Transaction` otomatis deref ke `&Connection`, jadi fungsi baca bisa dipanggil dengan `&tx` maupun `&conn`.

- [ ] **Step 1: Tambah dependensi**

```bash
cargo add uuid -p nabungo-storage --features v7
cargo add time -p nabungo-storage --features formatting
cargo add --dev time -p nabungo-storage --features macros
```

- [ ] **Step 2: Tulis test yang gagal** — `crates/storage/src/txn.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{NewAccount, get_account_ref, insert_account};
    use crate::category::{get_category_ref, insert_category};
    use crate::{now_utc, open_in_memory};
    use nabungo_core::{
        AccountKind, AccountRef, CategoryKind, CategoryRef, CurrencyCode, PostingDraft, TxnKind,
    };
    use time::macros::date;

    fn bank(tx: &Transaction, name: &str) -> AccountRef {
        let new = NewAccount { name: name.into(), kind: AccountKind::Bank, currency: CurrencyCode::IDR };
        let id = insert_account(tx, &new, &now_utc()).unwrap();
        get_account_ref(tx, id).unwrap().unwrap()
    }

    fn draft(kind: TxnKind, postings: Vec<(AccountRef, Option<CategoryRef>, i64)>) -> TxnDraft {
        TxnDraft {
            kind,
            date: date!(2026 - 10 - 12),
            note: None,
            postings: postings
                .into_iter()
                .map(|(account, category, amount)| PostingDraft { account, category, amount, memo: None })
                .collect(),
        }
    }

    #[test]
    fn account_ref_roundtrip() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let acc = bank(&tx, "BCA");
        assert_eq!(acc.kind, AccountKind::Bank);
        assert_eq!(acc.currency, CurrencyCode::IDR);
        assert_eq!(get_account_ref(&tx, Uuid::now_v7()).unwrap(), None);
    }

    #[test]
    fn category_ref_roundtrip() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let parent = insert_category(&tx, CategoryKind::Expense, "Makan & Minum", None).unwrap();
        let child = insert_category(&tx, CategoryKind::Expense, "Kopi", Some(parent)).unwrap();
        let r = get_category_ref(&tx, child).unwrap().unwrap();
        assert_eq!(r.kind, CategoryKind::Expense);
    }

    #[test]
    fn balance_is_sum_of_postings() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let (a, b) = (bank(&tx, "BCA"), bank(&tx, "Jago"));
        let now = now_utc();
        insert_txn(&tx, &draft(TxnKind::Opening, vec![(a, None, 500_000)]), &now).unwrap();
        insert_txn(&tx, &draft(TxnKind::Transfer, vec![(a, None, -200_000), (b, None, 200_000)]), &now)
            .unwrap();
        assert_eq!(account_balance(&tx, a.id).unwrap(), 300_000);
        assert_eq!(account_balance(&tx, b.id).unwrap(), 200_000);
    }

    #[test]
    fn soft_deleted_txn_excluded_and_restorable() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let a = bank(&tx, "BCA");
        let now = now_utc();
        let id = insert_txn(&tx, &draft(TxnKind::Opening, vec![(a, None, 500_000)]), &now).unwrap();

        assert!(set_deleted(&tx, id, Some(&now), &now).unwrap());
        assert_eq!(account_balance(&tx, a.id).unwrap(), 0);

        assert!(set_deleted(&tx, id, None, &now).unwrap());
        assert_eq!(account_balance(&tx, a.id).unwrap(), 500_000);

        assert!(!set_deleted(&tx, Uuid::now_v7(), Some(&now), &now).unwrap());
    }

    #[test]
    fn insert_txn_is_atomic_with_enclosing_transaction() {
        let mut conn = open_in_memory().unwrap();
        let a = {
            let tx = conn.transaction().unwrap();
            let a = bank(&tx, "BCA");
            tx.commit().unwrap();
            a
        };
        {
            let tx = conn.transaction().unwrap();
            insert_txn(&tx, &draft(TxnKind::Opening, vec![(a, None, 1)]), &now_utc()).unwrap();
            // drop tanpa commit = rollback
        }
        assert_eq!(account_balance(&conn, a.id).unwrap(), 0);
    }
}
```

Di `lib.rs` tambahkan `pub mod account; pub mod category; pub mod txn;` (buat `account.rs` & `category.rs` kosong agar modul terdaftar).

- [ ] **Step 3: Jalankan, pastikan gagal**

Run: `cargo test -p nabungo-storage txn::`
Expected: FAIL compile — `cannot find function insert_account`.

- [ ] **Step 4: Implementasi `lib.rs`** — tambahkan di bawah `use` yang ada:

```rust
pub use rusqlite;
```

tambahkan varian ke `StorageError`:

```rust
    #[error("corrupt data: {0}")]
    Corrupt(String),
```

dan fungsi:

```rust
/// Timestamp audit ISO-8601 UTC.
pub fn now_utc() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("RFC 3339 formatting of current time cannot fail")
}
```

- [ ] **Step 5: Implementasi `account.rs`**

```rust
use nabungo_core::{AccountKind, AccountRef, CurrencyCode};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use uuid::Uuid;

use crate::{Result, StorageError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAccount {
    pub name: String,
    pub kind: AccountKind,
    pub currency: CurrencyCode,
}

pub fn insert_account(tx: &Transaction, new: &NewAccount, now: &str) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO account (id, name, kind, currency, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![id.to_string(), new.name, new.kind.as_str(), new.currency.as_str(), now],
    )?;
    Ok(id)
}

pub fn get_account_ref(conn: &Connection, id: Uuid) -> Result<Option<AccountRef>> {
    let row = conn
        .query_row("SELECT kind, currency FROM account WHERE id = ?1", [id.to_string()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .optional()?;
    let Some((kind, currency)) = row else { return Ok(None) };
    let kind = AccountKind::parse(&kind)
        .ok_or_else(|| StorageError::Corrupt(format!("account {id}: unknown kind {kind:?}")))?;
    let currency = CurrencyCode::parse(&currency)
        .map_err(|e| StorageError::Corrupt(format!("account {id}: {e}")))?;
    Ok(Some(AccountRef { id, kind, currency }))
}
```

- [ ] **Step 6: Implementasi `category.rs`**

```rust
use nabungo_core::{CategoryKind, CategoryRef};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use uuid::Uuid;

use crate::{Result, StorageError};

pub fn insert_category(
    tx: &Transaction,
    kind: CategoryKind,
    name: &str,
    parent: Option<Uuid>,
) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO category (id, parent_id, kind, name) VALUES (?1, ?2, ?3, ?4)",
        params![id.to_string(), parent.map(|p| p.to_string()), kind.as_str(), name],
    )?;
    Ok(id)
}

pub fn get_category_ref(conn: &Connection, id: Uuid) -> Result<Option<CategoryRef>> {
    let kind = conn
        .query_row("SELECT kind FROM category WHERE id = ?1", [id.to_string()], |r| {
            r.get::<_, String>(0)
        })
        .optional()?;
    let Some(kind) = kind else { return Ok(None) };
    let kind = CategoryKind::parse(&kind)
        .ok_or_else(|| StorageError::Corrupt(format!("category {id}: unknown kind {kind:?}")))?;
    Ok(Some(CategoryRef { id, kind }))
}
```

- [ ] **Step 7: Implementasi `txn.rs`** (di atas modul test):

```rust
use nabungo_core::TxnDraft;
use nabungo_core::date::format_date;
use rusqlite::{Connection, Transaction, params};
use uuid::Uuid;

use crate::Result;

/// Simpan header + postings. Pemanggil WAJIB sudah menjalankan `nabungo_core::validate`.
pub fn insert_txn(tx: &Transaction, draft: &TxnDraft, now: &str) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO txn (id, kind, date, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![id.to_string(), draft.kind.as_str(), format_date(draft.date), draft.note, now],
    )?;
    let mut stmt = tx.prepare_cached(
        "INSERT INTO posting (id, txn_id, account_id, category_id, amount, memo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for p in &draft.postings {
        stmt.execute(params![
            Uuid::now_v7().to_string(),
            id.to_string(),
            p.account.id.to_string(),
            p.category.map(|c| c.id.to_string()),
            p.amount,
            p.memo,
        ])?;
    }
    Ok(id)
}

/// Soft delete (`Some(ts)`) atau restore (`None`). Mengembalikan `false` bila ID tidak ada.
pub fn set_deleted(tx: &Transaction, id: Uuid, deleted_at: Option<&str>, now: &str) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE txn SET deleted_at = ?2, updated_at = ?3 WHERE id = ?1",
        params![id.to_string(), deleted_at, now],
    )?;
    Ok(changed == 1)
}

/// Saldo = Σ postings dari transaksi aktif yang sudah `posted`.
pub fn account_balance(conn: &Connection, account_id: Uuid) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(p.amount), 0)
         FROM posting p JOIN txn t ON t.id = p.txn_id
         WHERE p.account_id = ?1 AND t.deleted_at IS NULL AND t.status = 'posted'",
        [account_id.to_string()],
        |r| r.get(0),
    )?)
}
```

- [ ] **Step 8: Jalankan, pastikan lulus**

Run: `cargo test -p nabungo-storage`
Expected: 9 passed (4 dari scaffold + 5 baru).

- [ ] **Step 9: Commit**

```bash
git add crates/storage
git commit -m "feat(storage): account, category and txn repositories with balance query"
```

---

### Task 6: `Ledger` di layer app + property test saldo

**Files:**
- Create: `crates/app/src/ledger.rs`, `crates/app/tests/ledger_balance.rs`
- Modify: `crates/app/src/lib.rs`, `crates/app/Cargo.toml`

**Interfaces:**
- Consumes: semua `Produces` dari Task 1–5.
- Produces:
  - `AppError { Storage(StorageError), Sqlite(rusqlite::Error), Txn(TxnError), NotFound(&'static str) }`
  - `PostingInput { account_id: Uuid, category_id: Option<Uuid>, amount: i64, memo: Option<String> }`
  - `TxnInput { kind: TxnKind, date: Date, note: Option<String>, postings: Vec<PostingInput> }` dengan `TxnInput::single(kind, date, account_id, category_id: Option<Uuid>, amount)` dan `TxnInput::transfer(date, from, to, amount: i64)` (amount positif)
  - `Ledger::open(&Path)`, `Ledger::open_in_memory()`, `create_account(NewAccount, opening_balance: i64, opening_date: Date) -> Result<Uuid, AppError>`, `create_category(CategoryKind, &str) -> Result<Uuid, AppError>`, `record(&TxnInput) -> Result<Uuid, AppError>`, `delete_txn(Uuid)`, `undo_delete_txn(Uuid)`, `balance(Uuid) -> Result<i64, AppError>`

- [ ] **Step 1: Tambah dependensi**

```bash
cargo add uuid time -p nabungo-app
cargo add --dev proptest -p nabungo-app
cargo add --dev time -p nabungo-app --features macros
```

- [ ] **Step 2: Tulis test yang gagal** — unit test di `crates/app/src/ledger.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use nabungo_core::{AccountKind, CurrencyCode};
    use time::macros::date;

    const D: Date = date!(2026 - 10 - 12);

    fn bank(ledger: &mut Ledger, opening: i64) -> Uuid {
        let new = NewAccount { name: "BCA".into(), kind: AccountKind::Bank, currency: CurrencyCode::IDR };
        ledger.create_account(new, opening, D).unwrap()
    }

    fn txn_count(ledger: &Ledger) -> i64 {
        ledger.conn.query_row("SELECT count(*) FROM txn", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn opening_balance_creates_opening_txn() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger, 1_000_000);
        assert_eq!(ledger.balance(acc).unwrap(), 1_000_000);
        assert_eq!(txn_count(&ledger), 1);
    }

    #[test]
    fn zero_opening_balance_creates_no_txn() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger, 0);
        assert_eq!(ledger.balance(acc).unwrap(), 0);
        assert_eq!(txn_count(&ledger), 0);
    }

    #[test]
    fn invalid_txn_is_rejected_and_not_stored() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger, 0);
        let err = ledger.record(&TxnInput::single(TxnKind::Expense, D, acc, None, -5_000)).unwrap_err();
        assert!(matches!(err, AppError::Txn(TxnError::MissingCategory)));
        assert_eq!(txn_count(&ledger), 0);
    }

    #[test]
    fn unknown_account_is_not_found() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let err = ledger
            .record(&TxnInput::single(TxnKind::Adjustment, D, Uuid::now_v7(), None, 1))
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound("account")));
    }

    #[test]
    fn delete_and_undo() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let acc = bank(&mut ledger, 0);
        let food = ledger.create_category(CategoryKind::Expense, "Makan").unwrap();
        let id = ledger.record(&TxnInput::single(TxnKind::Expense, D, acc, Some(food), -25_000)).unwrap();
        ledger.delete_txn(id).unwrap();
        assert_eq!(ledger.balance(acc).unwrap(), 0);
        ledger.undo_delete_txn(id).unwrap();
        assert_eq!(ledger.balance(acc).unwrap(), -25_000);
        assert!(matches!(ledger.delete_txn(Uuid::now_v7()), Err(AppError::NotFound("transaction"))));
    }
}
```

Integration property test `crates/app/tests/ledger_balance.rs` (acceptance "Saldo konsisten" & "Transfer seimbang"):

```rust
use nabungo_app::{Ledger, TxnInput};
use nabungo_core::{AccountKind, CategoryKind, CurrencyCode, TxnKind};
use nabungo_storage::account::NewAccount;
use proptest::prelude::*;
use time::macros::date;

#[derive(Debug, Clone)]
enum Op {
    Income(usize, i64),
    /// amount > 0 = pengeluaran, < 0 = refund
    Expense(usize, i64),
    Transfer(usize, usize, i64),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..3usize, 1i64..=10_000_000).prop_map(|(a, x)| Op::Income(a, x)),
        (0..3usize, -1_000_000i64..=10_000_000)
            .prop_filter("non-zero", |(_, x)| *x != 0)
            .prop_map(|(a, x)| Op::Expense(a, x)),
        (0..3usize, 0..3usize, 1i64..=10_000_000)
            .prop_filter("distinct accounts", |(a, b, _)| a != b)
            .prop_map(|(a, b, x)| Op::Transfer(a, b, x)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn balances_match_model(
        openings in prop::array::uniform3(-5_000_000i64..=5_000_000),
        ops in prop::collection::vec(op(), 0..300),
    ) {
        let d = date!(2026 - 10 - 12);
        let mut ledger = Ledger::open_in_memory().unwrap();
        let accounts: Vec<_> = openings
            .iter()
            .enumerate()
            .map(|(i, &o)| {
                let new = NewAccount { name: format!("A{i}"), kind: AccountKind::Bank, currency: CurrencyCode::IDR };
                ledger.create_account(new, o, d).unwrap()
            })
            .collect();
        let salary = ledger.create_category(CategoryKind::Income, "Gaji").unwrap();
        let food = ledger.create_category(CategoryKind::Expense, "Makan").unwrap();

        let mut model = openings;
        let mut transfers_only_total = model.iter().sum::<i64>();
        for op in &ops {
            let input = match *op {
                Op::Income(a, x) => {
                    model[a] += x;
                    transfers_only_total += x;
                    TxnInput::single(TxnKind::Income, d, accounts[a], Some(salary), x)
                }
                Op::Expense(a, x) => {
                    model[a] -= x;
                    transfers_only_total -= x;
                    TxnInput::single(TxnKind::Expense, d, accounts[a], Some(food), -x)
                }
                Op::Transfer(a, b, x) => {
                    model[a] -= x;
                    model[b] += x;
                    TxnInput::transfer(d, accounts[a], accounts[b], x)
                }
            };
            ledger.record(&input).unwrap();
        }

        for (i, id) in accounts.iter().enumerate() {
            prop_assert_eq!(ledger.balance(*id).unwrap(), model[i]);
        }
        // Transfer tidak mengubah total: total = opening + income − expense.
        let total: i64 = accounts.iter().map(|id| ledger.balance(*id).unwrap()).sum();
        prop_assert_eq!(total, transfers_only_total);
    }
}
```

Di `lib.rs` tambahkan `mod ledger;` dan `pub use ledger::{AppError, Ledger, PostingInput, TxnInput};`.

- [ ] **Step 3: Jalankan, pastikan gagal**

Run: `cargo test -p nabungo-app`
Expected: FAIL compile — `cannot find type Ledger`.

- [ ] **Step 4: Implementasi `ledger.rs`** (di atas modul test):

```rust
//! Use case pencatatan: resolve input berbasis ID → validasi domain → simpan atomik.

use std::path::Path;

use nabungo_core::{
    AccountRef, CategoryKind, PostingDraft, TxnDraft, TxnError, TxnKind, validate,
};
use nabungo_storage::account::{self, NewAccount};
use nabungo_storage::rusqlite::{self, Connection};
use nabungo_storage::{StorageError, category, now_utc, txn};
use time::Date;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Txn(#[from] TxnError),
    #[error("{0} not found")]
    NotFound(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingInput {
    pub account_id: Uuid,
    pub category_id: Option<Uuid>,
    pub amount: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnInput {
    pub kind: TxnKind,
    pub date: Date,
    pub note: Option<String>,
    pub postings: Vec<PostingInput>,
}

impl TxnInput {
    pub fn single(kind: TxnKind, date: Date, account_id: Uuid, category_id: Option<Uuid>, amount: i64) -> Self {
        Self {
            kind,
            date,
            note: None,
            postings: vec![PostingInput { account_id, category_id, amount, memo: None }],
        }
    }

    /// `amount` positif: keluar dari `from`, masuk ke `to`.
    pub fn transfer(date: Date, from: Uuid, to: Uuid, amount: i64) -> Self {
        Self {
            kind: TxnKind::Transfer,
            date,
            note: None,
            postings: vec![
                PostingInput { account_id: from, category_id: None, amount: -amount, memo: None },
                PostingInput { account_id: to, category_id: None, amount, memo: None },
            ],
        }
    }
}

pub struct Ledger {
    conn: Connection,
}

impl Ledger {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        Ok(Self { conn: nabungo_storage::open(path)? })
    }

    pub fn open_in_memory() -> Result<Self, AppError> {
        Ok(Self { conn: nabungo_storage::open_in_memory()? })
    }

    /// Buat akun; saldo awal ≠ 0 dicatat sebagai transaksi `opening`.
    pub fn create_account(
        &mut self,
        new: NewAccount,
        opening_balance: i64,
        opening_date: Date,
    ) -> Result<Uuid, AppError> {
        let now = now_utc();
        let tx = self.conn.transaction()?;
        let id = account::insert_account(&tx, &new, &now)?;
        if opening_balance != 0 {
            let draft = TxnDraft {
                kind: TxnKind::Opening,
                date: opening_date,
                note: None,
                postings: vec![PostingDraft {
                    account: AccountRef { id, kind: new.kind, currency: new.currency },
                    category: None,
                    amount: opening_balance,
                    memo: None,
                }],
            };
            validate(&draft)?;
            txn::insert_txn(&tx, &draft, &now)?;
        }
        tx.commit()?;
        Ok(id)
    }

    pub fn create_category(&mut self, kind: CategoryKind, name: &str) -> Result<Uuid, AppError> {
        let tx = self.conn.transaction()?;
        let id = category::insert_category(&tx, kind, name, None)?;
        tx.commit()?;
        Ok(id)
    }

    pub fn record(&mut self, input: &TxnInput) -> Result<Uuid, AppError> {
        let now = now_utc();
        let tx = self.conn.transaction()?;
        let draft = resolve(&tx, input)?;
        validate(&draft)?;
        let id = txn::insert_txn(&tx, &draft, &now)?;
        tx.commit()?;
        Ok(id)
    }

    pub fn delete_txn(&mut self, id: Uuid) -> Result<(), AppError> {
        let now = now_utc();
        self.set_deleted(id, Some(&now), &now)
    }

    pub fn undo_delete_txn(&mut self, id: Uuid) -> Result<(), AppError> {
        let now = now_utc();
        self.set_deleted(id, None, &now)
    }

    pub fn balance(&self, account_id: Uuid) -> Result<i64, AppError> {
        Ok(txn::account_balance(&self.conn, account_id)?)
    }

    fn set_deleted(&mut self, id: Uuid, deleted_at: Option<&str>, now: &str) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        if !txn::set_deleted(&tx, id, deleted_at, now)? {
            return Err(AppError::NotFound("transaction"));
        }
        tx.commit()?;
        Ok(())
    }
}

fn resolve(conn: &Connection, input: &TxnInput) -> Result<TxnDraft, AppError> {
    let postings = input
        .postings
        .iter()
        .map(|p| {
            let account = account::get_account_ref(conn, p.account_id)?
                .ok_or(AppError::NotFound("account"))?;
            let category = match p.category_id {
                Some(id) => Some(category::get_category_ref(conn, id)?.ok_or(AppError::NotFound("category"))?),
                None => None,
            };
            Ok(PostingDraft { account, category, amount: p.amount, memo: p.memo.clone() })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(TxnDraft { kind: input.kind, date: input.date, note: input.note.clone(), postings })
}
```

- [ ] **Step 5: Jalankan, pastikan lulus**

Run: `cargo test -p nabungo-app`
Expected: 5 unit test + 1 property test passed.

- [ ] **Step 6: Verifikasi seluruh workspace**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: tidak ada warning, semua test hijau.

- [ ] **Step 7: Commit**

```bash
git add crates/app
git commit -m "feat(app): Ledger use case with property-tested balances"
```

---

## Exit criteria Fase 1

- Semua test di atas hijau, clippy bersih.
- Acceptance PRD §5 yang terkunci: "Saldo konsisten" (property test), "Transfer seimbang" (validasi domain + property test), "Periode budget custom" (`period_of` test tabel + property test).
- Setelah ini: tulis plan Fase 2 (`docs/plans/YYYY-MM-DD-fase-2-ipc-shell.md`).
