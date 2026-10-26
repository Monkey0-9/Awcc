//! Zero-allocation branchless ASCII buffer parsers for high-frequency sysfs/procfs reading.

#[inline(always)]
pub fn fast_trim(mut bytes: &[u8]) -> &[u8] {
    while let Some((&first, rest)) = bytes.split_first() {
        if first == b' ' || first == b'\t' || first == b'\n' || first == b'\r' {
            bytes = rest;
        } else {
            break;
        }
    }
    while let Some((&last, rest)) = bytes.split_last() {
        if last == b' ' || last == b'\t' || last == b'\n' || last == b'\r' {
            bytes = rest;
        } else {
            break;
        }
    }
    bytes
}

/// Parses an unsigned 32-bit integer directly from an ASCII byte slice without heap allocation.
#[inline]
pub fn parse_u32_ascii(bytes: &[u8]) -> Option<u32> {
    let trimmed = fast_trim(bytes);
    if trimmed.is_empty() {
        return None;
    }
    let mut acc: u32 = 0;
    for &b in trimmed {
        if !b.is_ascii_digit() {
            return None;
        }
        acc = acc.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(acc)
}

/// Parses an unsigned 64-bit integer directly from an ASCII byte slice without heap allocation.
#[inline]
pub fn parse_u64_ascii(bytes: &[u8]) -> Option<u64> {
    let trimmed = fast_trim(bytes);
    if trimmed.is_empty() {
        return None;
    }
    let mut acc: u64 = 0;
    for &b in trimmed {
        if !b.is_ascii_digit() {
            return None;
        }
        acc = acc.checked_mul(10)?.checked_add((b - b'0') as u64)?;
    }
    Some(acc)
}

/// Parses a signed 32-bit integer directly from an ASCII byte slice.
#[inline]
pub fn parse_i32_ascii(bytes: &[u8]) -> Option<i32> {
    let trimmed = fast_trim(bytes);
    if trimmed.is_empty() {
        return None;
    }
    let (is_negative, digits) = if trimmed[0] == b'-' {
        (true, &trimmed[1..])
    } else if trimmed[0] == b'+' {
        (false, &trimmed[1..])
    } else {
        (false, trimmed)
    };

    if digits.is_empty() {
        return None;
    }

    let val = parse_u32_ascii(digits)? as i32;
    Some(if is_negative { -val } else { val })
}

/// Parses a floating-point number from ASCII bytes (e.g. "57.5" or "1250").
#[inline]
pub fn parse_f32_ascii(bytes: &[u8]) -> Option<f32> {
    let trimmed = fast_trim(bytes);
    if trimmed.is_empty() {
        return None;
    }

    if let Some(pos) = trimmed.iter().position(|&b| b == b'.') {
        let int_part = if pos > 0 {
            parse_i32_ascii(&trimmed[..pos])? as f32
        } else {
            0.0
        };
        let frac_bytes = &trimmed[pos + 1..];
        if frac_bytes.is_empty() {
            return Some(int_part);
        }
        let frac_val = parse_u32_ascii(frac_bytes)? as f32;
        let frac_div = 10f32.powi(frac_bytes.len() as i32);
        if int_part < 0.0 {
            Some(int_part - (frac_val / frac_div))
        } else {
            Some(int_part + (frac_val / frac_div))
        }
    } else {
        parse_i32_ascii(trimmed).map(|v| v as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_u32_ascii() {
        assert_eq!(parse_u32_ascii(b"12345\n"), Some(12345));
        assert_eq!(parse_u32_ascii(b"  4800  "), Some(4800));
        assert_eq!(parse_u32_ascii(b"0"), Some(0));
        assert_eq!(parse_u32_ascii(b"abc"), None);
        assert_eq!(parse_u32_ascii(b""), None);
    }

    #[test]
    fn test_parse_f32_ascii() {
        assert_eq!(parse_f32_ascii(b"57.5\n"), Some(57.5));
        assert_eq!(parse_f32_ascii(b"100"), Some(100.0));
        assert_eq!(parse_f32_ascii(b"0.25"), Some(0.25));
    }
}
