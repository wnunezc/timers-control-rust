//! Cuenta regresiva independiente de GUI, comandos y audio.
//!
//! El llamador proporciona `Instant` para pruebas deterministas. Los plazos son
//! absolutos: nunca se resta un segundo por cada mensaje de la GUI.

#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

/// Estado cerrado: solo `Timer` puede cambiar sus invariantes.
enum State {
    Running(Instant),
    Paused(Duration),
    Stopped(Duration),
    Finished,
}

/// Nombre inmutable y plazo monotónico o duración congelada.
pub struct Timer {
    name: String,
    state: State,
}

impl Timer {
    /// Crea e inicia una cuenta positiva. Rechaza nombres vacíos, espacios y
    /// controles para impedir, entre otros, NUL dentro de una futura cadena Win32.
    pub fn new(name: &str, duration: Duration, now: Instant) -> Result<Self, String> {
        if name.is_empty() || name.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err("El nombre no puede estar vacío ni contener espacios o controles".into());
        }
        if duration.is_zero() {
            return Err("La duración debe ser mayor que cero".into());
        }
        let deadline = now.checked_add(duration).ok_or("Duración fuera de rango")?;
        Ok(Self {
            name: name.into(),
            state: State::Running(deadline),
        })
    }

    /// Identificador sensible a mayúsculas, único dentro de `TimerApp`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Tiempo restante, saturado en cero si el plazo ya venció.
    pub fn remaining(&self, now: Instant) -> Duration {
        match self.state {
            State::Running(deadline) => deadline.saturating_duration_since(now),
            State::Paused(remaining) | State::Stopped(remaining) => remaining,
            State::Finished => Duration::ZERO,
        }
    }

    /// Plazo activo para el planificador; `None` evita despertares estando detenido.
    pub fn deadline(&self) -> Option<Instant> {
        match self.state {
            State::Running(deadline) => Some(deadline),
            _ => None,
        }
    }

    /// Etiqueta visible sin exponer la representación mutable del estado.
    pub fn label(&self) -> &'static str {
        match self.state {
            State::Running(_) => "en marcha",
            State::Paused(_) => "pausado",
            State::Stopped(_) => "detenido",
            State::Finished => "finalizado",
        }
    }

    /// Congela una cuenta en marcha; rechaza otros estados.
    pub fn pause(&mut self, now: Instant) -> Result<(), String> {
        match self.state {
            State::Running(_) => {
                self.state = State::Paused(self.remaining(now));
                Ok(())
            }
            _ => Err("El temporizador no está en marcha".into()),
        }
    }

    /// Reanuda una cuenta pausada o detenida desde su duración conservada.
    pub fn resume(&mut self, now: Instant) -> Result<(), String> {
        match self.state {
            State::Paused(remaining) | State::Stopped(remaining) => {
                self.state = State::Running(
                    now.checked_add(remaining)
                        .ok_or("Duración fuera de rango")?,
                );
                Ok(())
            }
            _ => Err("El temporizador no está pausado o detenido".into()),
        }
    }

    /// Detiene sin eliminar ni restablecer. Un finalizado permanece finalizado:
    /// `parar` y `continuar` no deben hacer sonar de nuevo una alarma vencida.
    pub fn stop(&mut self, now: Instant) {
        if !matches!(self.state, State::Finished) {
            self.state = State::Stopped(self.remaining(now));
        }
    }

    /// Registra el vencimiento exactamente una vez; no reproduce audio.
    pub fn expire(&mut self, now: Instant) -> bool {
        if matches!(self.state, State::Running(deadline) if deadline <= now) {
            self.state = State::Finished;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_enforces_domain_invariants() {
        let now = Instant::now();
        for name in ["", "dos palabras", "nul\0", "salto\n"] {
            assert!(Timer::new(name, Duration::from_secs(1), now).is_err());
        }
        assert!(Timer::new("uno", Duration::ZERO, now).is_err());
        assert!(Timer::new("uno", Duration::from_secs(u64::MAX), now).is_err());
        assert!(Timer::new("café", Duration::from_secs(1), now).is_ok());
    }

    #[test]
    fn pause_preserves_time_and_expiration_occurs_once() {
        let now = Instant::now();
        let mut timer = Timer::new("prueba", Duration::from_secs(10), now).unwrap();
        timer.pause(now + Duration::from_secs(3)).unwrap();
        assert_eq!(
            timer.remaining(now + Duration::from_secs(100)),
            Duration::from_secs(7)
        );
        assert!(timer.deadline().is_none());
        assert!(!timer.expire(now + Duration::from_secs(100)));
        timer.resume(now + Duration::from_secs(100)).unwrap();
        assert!(!timer.expire(now + Duration::from_secs(106)));
        assert!(timer.expire(now + Duration::from_secs(107)));
        assert!(!timer.expire(now + Duration::from_secs(108)));
    }

    #[test]
    fn stop_preserves_time_and_allows_resume_without_deleting() {
        let now = Instant::now();
        let mut timer = Timer::new("uno", Duration::from_secs(10), now).unwrap();
        timer.stop(now + Duration::from_secs(3));
        assert_eq!(timer.name(), "uno");
        assert_eq!(
            timer.remaining(now + Duration::from_secs(100)),
            Duration::from_secs(7)
        );
        assert!(!timer.expire(now + Duration::from_secs(100)));
        assert_eq!(timer.label(), "detenido");
        timer.resume(now + Duration::from_secs(100)).unwrap();
        assert!(timer.expire(now + Duration::from_secs(107)));
    }

    #[test]
    fn delayed_message_expires_once_and_finished_cannot_resume() {
        let now = Instant::now();
        let mut timer = Timer::new("prueba", Duration::from_secs(2), now).unwrap();
        assert!(timer.expire(now + Duration::from_secs(20)));
        assert_eq!(
            timer.remaining(now + Duration::from_secs(20)),
            Duration::ZERO
        );
        timer.stop(now + Duration::from_secs(21));
        assert_eq!(timer.label(), "finalizado");
        assert!(timer.resume(now + Duration::from_secs(22)).is_err());
        assert!(!timer.expire(now + Duration::from_secs(23)));
    }
}
