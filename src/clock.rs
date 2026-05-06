use core::time::Duration;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use anyhow::Result;
use better_default::Default;

#[derive(Default)]
struct Clock {
    #[default(Duration::from_nanos(238))]
    duration: Duration,
    cpu_clock: Option<Sender<()>>,
    ppu_clock: Option<Sender<()>>,
}

impl Clock {
    fn from_freq(frequency: f64) -> Clock {
        Clock {
            duration: Duration::from_nanos((1.0e9 / frequency) as u64),
            ..Default::default()
        }
    }

    fn start(&mut self) -> Result<()> {
        let (Some(cpu_clock), Some(ppu_clock)) = (self.cpu_clock.take(), self.ppu_clock.take())
        else {
            panic!("Didn't create cpu and ppu clocks before starting");
        };
        let mut cpu_counter = 4;
        loop {
            ppu_clock.send(())?;
            if cpu_counter == 4 {
                cpu_clock.send(())?;
                cpu_counter = 0;
            }
            cpu_counter += 1;
            thread::sleep(self.duration);
        }
    }
}
