use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub struct ThroughputMeter {
    packets: VecDeque<(Instant, usize)>,
    evict_record_duration: Duration,
}

impl Default for ThroughputMeter {
    fn default() -> Self {
        Self {
            packets: Default::default(),
            evict_record_duration: Duration::from_secs(60),
        }
    }
}

impl ThroughputMeter {
    pub fn add_record(&mut self, no_bytes: usize) {
        self.packets.push_back((Instant::now(), no_bytes));
        self.evict_stale_records();
    }

    pub fn evict_stale_records(&mut self) {
        while let Some((ins, _)) = self.packets.front() {
            if ins.elapsed() > self.evict_record_duration {
                self.packets.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn throughput(&self, d: Duration) -> usize {
        self.packets
            .iter()
            .rev()
            .filter(|(ins, _)| ins.elapsed() < d)
            .map(|(_, c)| *c)
            .sum()
    }
}
