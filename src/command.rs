//! Gramática de comandos. Devuelve datos tipados sin GUI, audio ni mutaciones.

#![forbid(unsafe_code)]

use crate::duration::parse_duration;
use std::time::Duration;

/// Los nombres se prestan de la entrada y no se copian al parsear.
pub enum Command<'a> {
    Empty,
    Help,
    Create { name: &'a str, duration: Duration },
    Configure { name: &'a str, duration: Duration },
    Pause(&'a str),
    Resume(&'a str),
    Stop(&'a str),
    Sound,
}

impl<'a> Command<'a> {
    /// Nombres y verbos distinguen mayúsculas. Rechaza argumentos extra.
    pub fn parse(input: &'a str) -> Result<Self, String> {
        let mut words = input.split_whitespace();
        let tokens = (words.next(), words.next(), words.next(), words.next());
        Ok(match tokens {
            (None, None, None, None) => Self::Empty,
            (Some("ayuda"), None, None, None) => Self::Help,
            (Some("sonido"), None, None, None) => Self::Sound,
            (Some("crear"), Some(name), Some(value), None) => Self::Create {
                name,
                duration: parse_duration(value)?,
            },
            (Some("configurar"), Some(name), Some(value), None) => Self::Configure {
                name,
                duration: parse_duration(value)?,
            },
            (Some("pausar"), Some(name), None, None) => Self::Pause(name),
            (Some("continuar"), Some(name), None, None) => Self::Resume(name),
            (Some("parar"), Some(name), None, None) => Self::Stop(name),
            _ => return Err("Comando inválido. Escribe ayuda".into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_and_extra_arguments() {
        for text in [
            "crear",
            "crear uno",
            "crear uno 1m extra",
            "pausar uno extra",
            "sonido extra",
            "borrar uno",
        ] {
            assert!(Command::parse(text).is_err(), "{text}");
        }
        assert!(matches!(Command::parse(" \t"), Ok(Command::Empty)));
        assert!(matches!(
            Command::parse("pausar café"),
            Ok(Command::Pause("café"))
        ));
    }
}
