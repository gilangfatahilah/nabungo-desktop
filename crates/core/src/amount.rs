//! Parser nominal ketikan user ("25rb", "1,5jt", "50rb-12.500") ke `i64` satuan minor.
use crate::txn::MAX_AMOUNT;

/// Alasan input nominal ditolak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AmountParseError {
    #[error("amount is empty")]
    Empty,
    #[error("amount is not a valid expression")]
    Invalid,
    #[error("amount must resolve to a whole number")]
    Fractional,
    #[error("amount exceeds the maximum of {MAX_AMOUNT}")]
    TooLarge,
}

/// Parse ekspresi nominal: `["-"] suku (("+"|"-") suku)*`, suku = `angka [rb|jt]`.
/// `.` = pemisah ribuan (grup tepat 3 digit), `,` = desimal. Hasil 0/negatif
/// dikembalikan apa adanya; menolak 0 adalah tugas `validate`.
pub fn parse_amount(input: &str) -> Result<i64, AmountParseError> {
    let s: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| b.to_ascii_lowercase())
        .collect();
    if s.is_empty() {
        return Err(AmountParseError::Empty);
    }
    let max = i128::from(MAX_AMOUNT);
    let mut pos = 0;
    let mut sign: i128 = 1;
    if s[0] == b'-' {
        sign = -1;
        pos = 1;
    }
    let mut total: i128 = 0;
    loop {
        let term = parse_term(&s, &mut pos)?;
        if term > max {
            return Err(AmountParseError::TooLarge);
        }
        total += sign * term; // |total| <= suku * jumlah suku, jauh di bawah i128
        if total.abs() > max {
            return Err(AmountParseError::TooLarge);
        }
        match s.get(pos) {
            None => break,
            Some(b'+') => sign = 1,
            Some(b'-') => sign = -1,
            Some(_) => return Err(AmountParseError::Invalid),
        }
        pos += 1;
    }
    i64::try_from(total).map_err(|_| AmountParseError::TooLarge)
}

fn digits(s: &[u8], pos: &mut usize) -> usize {
    let start = *pos;
    while s.get(*pos).is_some_and(u8::is_ascii_digit) {
        *pos += 1;
    }
    *pos - start
}

/// Nilai suku (non-negatif) setelah sufiks; ditolak bila tidak bulat.
fn parse_term(s: &[u8], pos: &mut usize) -> Result<i128, AmountParseError> {
    use AmountParseError::*;
    // Bagian bulat: digit+ ("." digit{3})*
    let start = *pos;
    if digits(s, pos) == 0 {
        return Err(Invalid);
    }
    let mut int_digits: Vec<u8> = s[start..*pos].to_vec();
    while s.get(*pos) == Some(&b'.') {
        *pos += 1;
        let g = *pos;
        if digits(s, pos) != 3 {
            return Err(Invalid);
        }
        int_digits.extend_from_slice(&s[g..*pos]);
    }
    // Bagian desimal opsional.
    let mut frac: &[u8] = &[];
    if s.get(*pos) == Some(&b',') {
        *pos += 1;
        let f = *pos;
        if digits(s, pos) == 0 {
            return Err(Invalid);
        }
        frac = &s[f..*pos];
    }
    let mult: i128 = match s.get(*pos..*pos + 2) {
        Some(b"rb") => 1_000,
        Some(b"jt") => 1_000_000,
        _ => 1,
    };
    if mult > 1 {
        *pos += 2;
    }
    // Sintaks salah (mis. "1,5,0jt") lebih diutamakan daripada galat nilai.
    if s.get(*pos)
        .is_some_and(|b| matches!(b, b',' | b'.' | b'0'..=b'9'))
    {
        return Err(Invalid);
    }
    let mut value: i128 = 0;
    for &d in &int_digits {
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(i128::from(d - b'0')))
            .ok_or(TooLarge)?;
    }
    value = value.checked_mul(mult).ok_or(TooLarge)?;
    // Desimal: nol di ujung tak berpengaruh; sisanya harus habis dibagi.
    let frac = match frac.iter().rposition(|&d| d != b'0') {
        Some(i) => &frac[..=i],
        None => &[],
    };
    if !frac.is_empty() {
        // mult <= 10^6, jadi lebih dari 6 digit signifikan pasti tidak bulat.
        if frac.len() > 6 {
            return Err(Fractional);
        }
        let f = frac
            .iter()
            .fold(0i128, |a, &d| a * 10 + i128::from(d - b'0'));
        let scaled = f * mult;
        let div = 10i128.pow(frac.len() as u32);
        if scaled % div != 0 {
            return Err(Fractional);
        }
        value = value.checked_add(scaled / div).ok_or(TooLarge)?;
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_table() {
        let ok = [
            ("25000", 25_000),
            ("25.000", 25_000),
            ("1.250.000", 1_250_000),
            ("25rb", 25_000),
            ("25 RB", 25_000),
            ("1,5jt", 1_500_000),
            ("1,25rb", 1_250),
            ("2jt", 2_000_000),
            ("25000+12500", 37_500),
            ("50rb - 12.500", 37_500),
            ("-5000", -5_000),
            ("1jt+250rb+3.000", 1_253_000),
            ("0", 0),
        ];
        for (input, want) in ok {
            assert_eq!(parse_amount(input), Ok(want), "{input}");
        }
    }

    #[test]
    fn rejects_table() {
        use AmountParseError::*;
        let bad = [
            ("", Empty),
            ("   ", Empty),
            ("1.5", Invalid),
            ("25.00", Invalid),
            ("1.2345", Invalid),
            ("abc", Invalid),
            ("25++1", Invalid),
            ("25+", Invalid),
            ("rb", Invalid),
            ("1,5,0jt", Invalid),
            ("25.000,50", Fractional),
            ("10,5", Fractional),
            ("1,2345rb", Fractional),
            ("1000001jt", TooLarge),
            ("999999999999999999999999", TooLarge),
        ];
        for (input, want) in bad {
            assert_eq!(parse_amount(input), Err(want), "{input}");
        }
    }

    #[test]
    fn max_amount_boundary() {
        assert_eq!(parse_amount("1.000.000jt"), Ok(MAX_AMOUNT));
        assert_eq!(
            parse_amount("1.000.000jt+1"),
            Err(AmountParseError::TooLarge)
        );
    }
}
