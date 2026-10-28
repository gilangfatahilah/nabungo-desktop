//! Auto-capture transaksi dari email notifikasi (EML-01–EML-03): parsing pesan, rule engine
//! murni, dan sumber IMAP. Satu-satunya crate dengan kode jaringan; app tidak memanggilnya
//! sampai user mengonfigurasi email.

pub mod defaults;
pub mod message;
pub mod rule;
pub mod source;
pub mod suggest;
