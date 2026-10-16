//! Parser de duraciones ASCII con aritmética comprobada y sin asignaciones auxiliares.

#![forbid(unsafe_code)]

use std::time::Duration;

/// Acepta MM:SS, HH:MM:SS, MMSS, HHMMSS, xh, xm y xs.
///
/// Los campos de reloj tienen dos dígitos y minutos/segundos entre 00 y 59.
/// Las unidades permiten cantidades mayores (por ejemplo, `90m`). Rechaza cero,
/// signos, Unicode y desbordamientos sin hacer slicing UTF-8 inseguro.
pub fn parse_duration(text: &str) -> Result<Duration, String> {
    const ERROR: &str =
        "Duración inválida: MM:SS, HH:MM:SS, MMSS, HHMMSS, xh, xm o xs; mayor que cero";
    if !text.is_ascii() {
        return Err(ERROR.into());
    }
    let mut parts = text.split(':');
    let first = parts.next().unwrap_or("");
    let second = parts.next();
    let third = parts.next();
    let components = match (second, third) {
        (Some(m), Some(s)) if parts.next().is_none() => Some((first, m, s)),
        (Some(s), None) => Some(("00", first, s)),
        (None, None) if text.bytes().all(|b| b.is_ascii_digit()) => match text.len() {
            4 => Some(("00", &text[..2], &text[2..])),
            6 => Some((&text[..2], &text[2..4], &text[4..])),
            _ => return Err(ERROR.into()),
        },
        (None, None) => None,
        _ => return Err(ERROR.into()),
    };
    let seconds = if let Some((hours, minutes, seconds)) = components {
        if hours.len() != 2 || minutes.len() != 2 || seconds.len() != 2 {
            return Err(ERROR.into());
        }
        let h = digits(hours).ok_or(ERROR)?;
        let m = digits(minutes).filter(|&n| n < 60).ok_or(ERROR)?;
        let s = digits(seconds).filter(|&n| n < 60).ok_or(ERROR)?;
        h.checked_mul(3600)
            .and_then(|n| n.checked_add(m * 60 + s))
            .ok_or(ERROR)?
    } else {
        let (number, unit) = text.split_at(text.len().checked_sub(1).ok_or(ERROR)?);
        let factor = match unit {
            "s" => 1,
            "m" => 60,
            "h" => 3600,
            _ => return Err(ERROR.into()),
        };
        digits(number)
            .and_then(|n| n.checked_mul(factor))
            .ok_or(ERROR)?
    };
    if seconds == 0 {
        return Err(ERROR.into());
    }
    Ok(Duration::from_secs(seconds))
}

fn digits(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_unicode_zero_and_overflow() {
        for text in [
            "",
            "0s",
            "0000",
            "00:00:00",
            "-1s",
            "+1s",
            "2",
            "1:60:0",
            "1:0",
            "18446744073709551615h",
            "💡",
            "0060",
            "006000",
            "00:00:01:02",
            "1💡s",
            "000",
            "0000000",
        ] {
            assert!(parse_duration(text).is_err(), "{text}");
        }
    }

    #[test]
    fn accepts_all_requested_duration_formats() {
        for (input, seconds) in [
            ("01:30", 90),
            ("01:02:03", 3723),
            ("0130", 90),
            ("010203", 3723),
            ("2h", 7200),
            ("90m", 5400),
            ("30s", 30),
            ("99:59:59", 359999),
        ] {
            assert_eq!(
                parse_duration(input).unwrap(),
                Duration::from_secs(seconds),
                "{input}"
            );
        }
    }
}
