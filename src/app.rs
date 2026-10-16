//! Casos de uso y planificación independientes de Windows y del reproductor.
//!
//! Devuelve efectos como datos. El adaptador muestra el resultado y reproduce el
//! audio: las pruebas del dominio no necesitan ventanas ni dispositivos.

#![forbid(unsafe_code)]

use crate::command::Command;
use crate::timers::Timer;
use std::time::{Duration, Instant};

/// Resultado de un comando; el adaptador ejecuta el audio si `sound` es true.
pub struct Outcome {
    pub status: String,
    pub sound: bool,
}

/// Colección encapsulada en orden de creación, con nombres únicos.
#[derive(Default)]
pub struct TimerApp {
    timers: Vec<Timer>,
}

impl TimerApp {
    /// Ejecuta un comando. Los errores no reemplazan cuentas existentes.
    pub fn execute(&mut self, command: Command<'_>, now: Instant) -> Result<Outcome, String> {
        let mut sound = false;
        let status = match command {
            Command::Empty => String::new(),
            Command::Help => "crear NOMBRE 05:00 | configurar NOMBRE 1h\npausar NOMBRE | continuar NOMBRE | parar NOMBRE | sonido".into(),
            Command::Create { name, duration } => {
                if self.timers.iter().any(|timer| timer.name() == name) { return Err("Ese nombre ya existe".into()); }
                self.timers.push(Timer::new(name, duration, now)?);
                format!("Creado: {name}")
            }
            Command::Configure { name, duration } => {
                let replacement = Timer::new(name, duration, now)?;
                *self.find_mut(name)? = replacement;
                format!("Configurado e iniciado: {name}")
            }
            Command::Pause(name) => { self.find_mut(name)?.pause(now)?; format!("pausar: {name}") }
            Command::Resume(name) => { self.find_mut(name)?.resume(now)?; format!("continuar: {name}") }
            Command::Stop(name) => { self.find_mut(name)?.stop(now); format!("parar: {name}") }
            Command::Sound => { sound = true; "Audio del sistema solicitado".into() }
        };
        Ok(Outcome { status, sound })
    }

    /// Búsqueda única compartida por las operaciones que modifican cuentas.
    fn find_mut(&mut self, name: &str) -> Result<&mut Timer, String> {
        self.timers
            .iter_mut()
            .find(|timer| timer.name() == name)
            .ok_or_else(|| "No existe ese temporizador".into())
    }

    /// Detecta vencimientos una sola vez. Conserva la conducta inicial: una
    /// notificación y un sonido por lote detectado junto, sin cola ni mezcla.
    pub fn advance(&mut self, now: Instant) -> Option<Outcome> {
        let mut names = String::new();
        for timer in &mut self.timers {
            if timer.expire(now) {
                if !names.is_empty() {
                    names.push_str(", ");
                }
                names.push_str(timer.name());
            }
        }
        if names.is_empty() {
            None
        } else {
            Some(Outcome {
                status: format!("Finalizado: {names}"),
                sound: true,
            })
        }
    }

    /// Texto con segundos redondeados hacia arriba y un instante común para filas.
    pub fn rows(&self, now: Instant) -> Vec<String> {
        self.timers
            .iter()
            .map(|timer| {
                let left = timer.remaining(now);
                let seconds = left.as_secs() + u64::from(left.subsec_nanos() != 0);
                format!(
                    "{:02}:{:02}:{:02}  {}  [{}]",
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60,
                    timer.name(),
                    timer.label()
                )
            })
            .collect()
    }

    /// Próximo cambio de segundo visible o vencimiento si está minimizado.
    /// `None` indica que no hay cuentas activas y no se necesita timer del SO.
    pub fn next_wake(&self, now: Instant, minimized: bool) -> Option<Duration> {
        self.timers
            .iter()
            .filter_map(|timer| {
                let left = timer.deadline()?.saturating_duration_since(now);
                Some(if minimized {
                    left
                } else {
                    let fraction = left.subsec_nanos();
                    if fraction == 0 {
                        left.min(Duration::from_secs(1))
                    } else {
                        Duration::from_nanos(fraction as u64)
                    }
                })
            })
            .min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execute(app: &mut TimerApp, text: &str, now: Instant) -> Result<Outcome, String> {
        app.execute(Command::parse(text)?, now)
    }

    #[test]
    fn commands_affect_only_the_named_timer_and_preserve_order() {
        let now = Instant::now();
        let mut app = TimerApp::default();
        execute(&mut app, "crear uno 10s", now).unwrap();
        execute(&mut app, "crear dos 20s", now).unwrap();
        execute(&mut app, "pausar uno", now + Duration::from_secs(3)).unwrap();
        assert_eq!(
            app.rows(now + Duration::from_secs(8)),
            ["00:00:07  uno  [pausado]", "00:00:12  dos  [en marcha]"]
        );
        assert!(execute(&mut app, "crear uno 1h", now).is_err());
        assert!(execute(&mut app, "pausar inexistente", now).is_err());
    }

    #[test]
    fn configuration_restarts_and_invalid_input_preserves_state() {
        let now = Instant::now();
        let mut app = TimerApp::default();
        execute(&mut app, "crear uno 10s", now).unwrap();
        execute(&mut app, "parar uno", now + Duration::from_secs(2)).unwrap();
        assert!(execute(&mut app, "configurar uno 💡", now).is_err());
        assert_eq!(app.rows(now), ["00:00:08  uno  [detenido]"]);
        execute(&mut app, "configurar uno 5s", now).unwrap();
        assert_eq!(app.rows(now), ["00:00:05  uno  [en marcha]"]);
    }

    #[test]
    fn simultaneous_expiration_produces_one_effect_once() {
        let now = Instant::now();
        let mut app = TimerApp::default();
        execute(&mut app, "crear uno 2s", now).unwrap();
        execute(&mut app, "crear dos 3s", now).unwrap();
        let outcome = app.advance(now + Duration::from_secs(10)).unwrap();
        assert_eq!(outcome.status, "Finalizado: uno, dos");
        assert!(outcome.sound);
        assert!(app.advance(now + Duration::from_secs(20)).is_none());
        assert!(app.next_wake(now, false).is_none());
    }

    #[test]
    fn scheduler_sleeps_when_idle_and_uses_deadline_when_minimized() {
        let now = Instant::now();
        let mut app = TimerApp::default();
        assert_eq!(app.next_wake(now, false), None);
        execute(&mut app, "crear uno 10s", now).unwrap();
        let later = now + Duration::from_millis(1250);
        assert_eq!(
            app.next_wake(later, false),
            Some(Duration::from_millis(750))
        );
        assert_eq!(
            app.next_wake(later, true),
            Some(Duration::from_millis(8750))
        );
        execute(&mut app, "pausar uno", later).unwrap();
        assert_eq!(app.next_wake(later, false), None);
    }

    #[test]
    fn sound_request_is_data_without_playback() {
        assert!(
            execute(&mut TimerApp::default(), "sonido", Instant::now())
                .unwrap()
                .sound
        );
    }
}
