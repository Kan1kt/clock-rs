use std::{
    cell::Cell,
    process,
    time::{Duration, Instant},
};

use crate::state::State;

#[derive(Clone, Copy, PartialEq)]
pub enum TimerPhase {
    Work,
    Rest,
    Prep,
}

pub enum CounterType {
    Stopwatch,
    Timer { duration: Duration, kill: bool },
    Pomodoro {
        work: Duration,
        rest: Duration,
        prep: Duration,
        phase: Cell<TimerPhase>,
        cycles: Cell<u32>,
        current_target: Cell<Duration>,
    },
}

pub struct Counter {
    pub text: &'static str,
    pub ty: CounterType, // Mudado para pub para acessarmos as fases no mod.rs
    start: Cell<Instant>,
    last_pause: Cell<Option<Instant>>,
    pub paused: bool,
}

impl Counter {
    pub const DEFAULT_TIMER_DURATION: u64 = 5 * 60;
    pub const MAX_TIMER_DURATION: u64 = 99 * 3600 + 59 * 60 + 59;
    const TEXT: &'static str = "P: Play/Pause, R: Restart, X: Reset Ciclos";
    const TEXT_PAUSED: &'static str = "P: Play/Pause, R: Restart [PAUSADO]";

    pub fn new(ty: CounterType) -> Self {
        Self {
            text: Self::TEXT,
            ty,
            start: Cell::new(Instant::now()),
            last_pause: Cell::new(None),
            paused: false,
        }
    }

    pub fn toggle_pause(&mut self) {
        self.text = if self.paused {
            if let Some(last_pause) = self.last_pause.get() {
                self.start.set(self.start.get() + last_pause.elapsed());
                self.last_pause.set(None);
            }
            Self::TEXT
        } else {
            self.last_pause.set(Some(Instant::now()));
            Self::TEXT_PAUSED
        };

        self.paused = !self.paused;
    }

    pub fn restart(&mut self) {
        self.start.set(Instant::now());
        self.last_pause.set(None);

        if let CounterType::Pomodoro { phase, current_target, work, .. } = &self.ty {
            phase.set(TimerPhase::Work);
            current_target.set(*work);
        }

        if self.paused {
            self.toggle_pause();
        }
    }

    pub fn get_time(&self) -> (u32, u32, u32) {
        let mut elapsed = if self.paused {
            match self.last_pause.get() {
                Some(last_pause) => last_pause.duration_since(self.start.get()),
                _ => Duration::from_secs(0),
            }
        } else {
            self.start.get().elapsed()
        };

        let mut secs = elapsed.as_secs() as u32;

        match &self.ty {
            CounterType::Stopwatch => {}
            CounterType::Timer { duration, kill } => {
                elapsed = duration.saturating_sub(elapsed.saturating_sub(Duration::from_secs(1)));
                secs = elapsed.as_secs() as u32;

                if secs == 0 && *kill {
                    State::exit();
                    process::exit(0);
                }
            }
            // --- NOSSA LÓGICA DO POMODORO ---
            CounterType::Pomodoro { work, rest, prep, phase, cycles, current_target } => {
                let target = current_target.get();
                
                if elapsed >= target {
                    print!("\x07"); // Emite o bip do terminal
                    let _ = std::io::Write::flush(&mut std::io::stdout());

                    let (next_phase, next_target) = match phase.get() {
                        TimerPhase::Work => {
                            cycles.set(cycles.get() + 1);
                            (TimerPhase::Rest, *rest)
                        }
                        TimerPhase::Rest => (TimerPhase::Prep, *prep),
                        TimerPhase::Prep => (TimerPhase::Work, *work),
                    };

                    phase.set(next_phase);
                    current_target.set(next_target);
                    self.start.set(Instant::now());
                    elapsed = Duration::from_secs(0);
                }

                let remaining = current_target.get().saturating_sub(elapsed);
                secs = remaining.as_secs() as u32;
            }
        }

        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        let seconds = secs % 60;

        (hours, minutes, seconds)
    }
}
