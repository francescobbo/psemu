#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Increasing,
    Decreasing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeRate {
    Linear,
    Exponential,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AdsrEnvelope {
    pub phase: AdsrPhase,
    pub level: i16,
    sustain_level: u16,
    counter: u32,

    settings: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AdsrPhase {
    Attack,
    Decay,
    Sustain,
    #[default]
    Release,
}

impl AdsrEnvelope {
    pub fn write_low(&mut self, value: u16) {
        self.settings = (self.settings & 0xFFFF_0000) | u32::from(value);
    }

    pub fn write_high(&mut self, value: u16) {
        self.settings = (self.settings & 0x0000_FFFF) | (u32::from(value) << 16);
    }

    pub fn read_low(&self) -> u16 {
        (self.settings & 0xFFFF) as u16
    }

    pub fn read_high(&self) -> u16 {
        (self.settings >> 16) as u16
    }

    pub fn key_on(&mut self) {
        self.level = 0;
        self.phase = AdsrPhase::Attack;
    }

    pub fn key_off(&mut self) {
        self.phase = AdsrPhase::Release;
        println!("Key off, transitioning to Release phase");
    }

    fn sustain_level(&self) -> u16 {
        (((self.settings & 0xf) + 1) * 0x800) as u16
    }

    fn check_for_phase_transition(&mut self) {
        if self.phase == AdsrPhase::Attack && self.level == 0x7FFF {
            println!("Transitioning from Attack to Decay");
            self.phase = AdsrPhase::Decay;
        }

        // Note that the envelope should go straight from Attack to Sustain if the sustain level is
        // set to the highest possible value
        if self.phase == AdsrPhase::Decay && (self.level as u16) <= self.sustain_level() {
            println!("Transitioning from Decay to Sustain");
            self.phase = AdsrPhase::Sustain;
        }
    }

    fn direction(&self) -> Direction {
        match self.phase {
            AdsrPhase::Attack => Direction::Increasing,
            AdsrPhase::Decay | AdsrPhase::Release => Direction::Decreasing,
            AdsrPhase::Sustain => if self.settings & (1 << 30) != 0 {
                Direction::Decreasing
            } else {
                Direction::Increasing
            },
        }
    }

    fn change_rate(&self) -> ChangeRate {
        match self.phase {
            AdsrPhase::Decay => ChangeRate::Exponential,
            AdsrPhase::Release => if self.settings & (1 << 21) != 0 {
                ChangeRate::Exponential
            } else {
                ChangeRate::Linear
            },
            AdsrPhase::Attack => if self.settings & (1 << 15) != 0 {
                ChangeRate::Exponential
            } else {
                ChangeRate::Linear
            },
            AdsrPhase::Sustain => if self.settings & (1 << 31) != 0 {
                ChangeRate::Exponential
            } else {
                ChangeRate::Linear
            },
        }
    }

    fn shift(&self) -> u8 {
        // The shift value is determined by the settings bits 11-14
        let bit = match self.phase {
            AdsrPhase::Attack => 10,
            AdsrPhase::Decay => 4,
            AdsrPhase::Sustain => 24,
            AdsrPhase::Release => 16,
        };
        ((self.settings >> bit) & 0x1f) as u8
    }

    fn step(&self) -> u8 {
        // The step value is determined by the settings bits 0-3
        let val = match self.phase {
            AdsrPhase::Decay => 0,
            AdsrPhase::Release => 0,
            AdsrPhase::Attack => (self.settings >> 8) & 3,
            AdsrPhase::Sustain => (self.settings >> 20) & 3,
        };

        val as u8
    }

    pub fn clock(&mut self) {
        self.check_for_phase_transition();

        let direction = self.direction();
        let rate = self.change_rate();
        let shift = self.shift();
        let mut step = i32::from(7 - self.step());

        if direction == Direction::Decreasing {
            step = !step;
        }

        let shift = if direction == Direction::Increasing && rate == ChangeRate::Exponential && self.level > 0x6000 {
            shift + 2
        } else {
            shift
        };

        // For a shift value of N, the envelope should update every 1 << (N - 11) cycles.
        // Accomplish this by using a counter decrement of MAX >> (N - 11)
        step <<= 11_u8.saturating_sub(shift);

        let prev_level: i32 = self.level.into();
        if direction == Direction::Decreasing && rate == ChangeRate::Exponential {
            step = (step * prev_level) >> 15;
        }

        let counter_shift = shift.saturating_sub(11);
        let mut counter_increment = if counter_shift < 16 { 0x8000 >> counter_shift } else { 0 };

        if counter_increment == 0 && (step != 3 || self.shift() != 31) {
            counter_increment = 1;
        }

        self.counter += counter_increment;
        // Check bit 15 of the counter to determine if the envelope level should update
        if self.counter & 0x8000 == 0 {
            // Envelope level does not update this cycle
            return;
        }

        self.counter = 0;
        let new_level = prev_level + step;
        self.level = match direction {
            Direction::Increasing => {
                new_level.clamp(i16::MIN.into(), i16::MAX.into()) as i16
            }
            Direction::Decreasing => {
                new_level.clamp(0, i16::MAX.into()) as i16
            }
        };
    }
}