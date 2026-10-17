//! Kategori default (CAT-01), di-seed saat onboarding dalam bahasa yang dipilih.

use nabungo_core::CategoryKind;
use nabungo_storage::category::{self, NewCategory};
use nabungo_storage::rusqlite::Transaction;

use crate::{AppError, Locale};

/// (nama ID, nama EN)
type Name = (&'static str, &'static str);

struct Root {
    kind: CategoryKind,
    name: Name,
    icon: &'static str,
    children: &'static [Name],
}

const fn expense(name: Name, icon: &'static str, children: &'static [Name]) -> Root {
    Root {
        kind: CategoryKind::Expense,
        name,
        icon,
        children,
    }
}

const fn income(name: Name, icon: &'static str) -> Root {
    Root {
        kind: CategoryKind::Income,
        name,
        icon,
        children: &[],
    }
}

const DEFAULTS: &[Root] = &[
    expense(
        ("Makan & Minum", "Food & Drink"),
        "utensils",
        &[
            ("Restoran", "Restaurants"),
            ("Belanja dapur", "Groceries"),
            ("Kopi & jajan", "Coffee & Snacks"),
        ],
    ),
    expense(
        ("Transportasi", "Transportation"),
        "car",
        &[
            ("Bensin", "Fuel"),
            ("Transportasi umum", "Public Transit"),
            ("Ojek online", "Ride-hailing"),
            ("Parkir & tol", "Parking & Tolls"),
        ],
    ),
    expense(
        ("Tagihan", "Bills"),
        "receipt",
        &[
            ("Listrik", "Electricity"),
            ("Air", "Water"),
            ("Internet", "Internet"),
            ("Pulsa & data", "Mobile Plan"),
        ],
    ),
    expense(
        ("Belanja", "Shopping"),
        "shopping-bag",
        &[
            ("Pakaian", "Clothing"),
            ("Elektronik", "Electronics"),
            ("Rumah tangga", "Household"),
        ],
    ),
    expense(
        ("Kesehatan", "Health"),
        "heart-pulse",
        &[
            ("Dokter & obat", "Doctor & Medicine"),
            ("Asuransi", "Insurance"),
        ],
    ),
    expense(
        ("Hiburan", "Entertainment"),
        "clapperboard",
        &[
            ("Langganan", "Subscriptions"),
            ("Hobi", "Hobbies"),
            ("Liburan", "Travel"),
        ],
    ),
    expense(("Pendidikan", "Education"), "graduation-cap", &[]),
    expense(("Keluarga", "Family"), "users", &[]),
    expense(("Biaya admin", "Fees"), "landmark", &[]),
    expense(("Lainnya", "Other"), "circle-ellipsis", &[]),
    income(("Gaji", "Salary"), "wallet"),
    income(("Bonus", "Bonus"), "gift"),
    income(("Bunga & dividen", "Interest & Dividends"), "trending-up"),
    income(("Hadiah", "Gifts"), "hand-heart"),
    income(("Lainnya", "Other"), "circle-ellipsis"),
];

/// Seed kategori default; no-op (`false`) bila sudah ada kategori apa pun.
pub(crate) fn seed_default_categories(tx: &Transaction, locale: Locale) -> Result<bool, AppError> {
    let any: bool = tx.query_row("SELECT EXISTS (SELECT 1 FROM category)", [], |r| r.get(0))?;
    if any {
        return Ok(false);
    }
    let pick = |(id, en): Name| match locale {
        Locale::Id => id,
        Locale::En => en,
    };
    for (order, root) in (0i64..).zip(DEFAULTS) {
        let mut new = NewCategory::new(root.kind, pick(root.name), None);
        new.icon = Some(root.icon.to_owned());
        new.sort_order = order;
        let parent = category::insert_category(tx, &new)?;
        for (child_order, &child) in (0i64..).zip(root.children) {
            let mut new = NewCategory::new(root.kind, pick(child), Some(parent));
            new.icon = Some(root.icon.to_owned());
            new.sort_order = child_order;
            category::insert_category(tx, &new)?;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ledger;
    use nabungo_core::CategoryKind;

    fn count(ledger: &Ledger, sql: &str) -> i64 {
        ledger.conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn seed_in_locale_once() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let tx = ledger.conn.transaction().unwrap();
        assert!(seed_default_categories(&tx, Locale::En).unwrap());
        assert!(!seed_default_categories(&tx, Locale::Id).unwrap());
        tx.commit().unwrap();

        let roots = "SELECT count(*) FROM category WHERE parent_id IS NULL";
        assert_eq!(count(&ledger, roots), 15);
        assert_eq!(count(&ledger, &format!("{roots} AND kind = 'expense'")), 10);
        assert_eq!(count(&ledger, &format!("{roots} AND kind = 'income'")), 5);
        assert_eq!(
            count(
                &ledger,
                "SELECT count(*) FROM category WHERE parent_id IS NOT NULL"
            ),
            19
        );
        let fees = ledger.list_categories().unwrap();
        assert!(
            fees.iter().any(|c| c.name == "Fees"
                && c.kind == CategoryKind::Expense
                && c.parent_id.is_none())
        );
    }

    #[test]
    fn seed_indonesian_names() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let tx = ledger.conn.transaction().unwrap();
        assert!(seed_default_categories(&tx, Locale::Id).unwrap());
        tx.commit().unwrap();
        let all = ledger.list_categories().unwrap();
        let food = all.iter().find(|c| c.name == "Makan & Minum").unwrap();
        assert_eq!(food.icon.as_deref(), Some("utensils"));
        let coffee = all.iter().find(|c| c.name == "Kopi & jajan").unwrap();
        assert_eq!(coffee.parent_id, Some(food.id));
        assert_eq!(coffee.icon.as_deref(), Some("utensils"));
    }
}
