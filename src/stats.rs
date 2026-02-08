/// Rolling statistics for population tracking and graph display.

/// Ring buffer that stores the last N samples of a metric.
pub struct RingBuffer {
    data: Vec<f32>,
    head: usize,
    len: usize,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0.0; capacity],
            head: 0,
            len: 0,
            capacity,
        }
    }

    pub fn push(&mut self, value: f32) {
        self.data[self.head] = value;
        self.head = (self.head + 1) % self.capacity;
        if self.len < self.capacity {
            self.len += 1;
        }
    }

    /// Return samples in chronological order.
    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        let start = if self.len < self.capacity {
            0
        } else {
            self.head
        };
        (0..self.len).map(move |i| self.data[(start + i) % self.capacity])
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn last(&self) -> Option<f32> {
        if self.len == 0 {
            None
        } else {
            let idx = (self.head + self.capacity - 1) % self.capacity;
            Some(self.data[idx])
        }
    }
}

/// All tracked simulation statistics.
pub struct SimStats {
    pub population: RingBuffer,
    pub avg_energy: RingBuffer,
    pub food_count: RingBuffer,
    pub births: RingBuffer,
    pub deaths: RingBuffer,
    pub avg_generation: RingBuffer,

    // Per-tick accumulators
    pub births_this_tick: u32,
    pub deaths_this_tick: u32,
    pub sample_interval: u32,
    pub tick_counter: u32,
}

impl SimStats {
    pub fn new(capacity: usize) -> Self {
        Self {
            population: RingBuffer::new(capacity),
            avg_energy: RingBuffer::new(capacity),
            food_count: RingBuffer::new(capacity),
            births: RingBuffer::new(capacity),
            deaths: RingBuffer::new(capacity),
            avg_generation: RingBuffer::new(capacity),
            births_this_tick: 0,
            deaths_this_tick: 0,
            sample_interval: 10, // sample every N ticks
            tick_counter: 0,
        }
    }

    /// Record a sample from the current simulation state.
    pub fn record(
        &mut self,
        entity_count: usize,
        avg_energy: f32,
        food_count: usize,
        avg_generation: f32,
    ) {
        self.tick_counter += 1;
        if self.tick_counter % self.sample_interval != 0 {
            return;
        }

        self.population.push(entity_count as f32);
        self.avg_energy.push(avg_energy);
        self.food_count.push(food_count as f32);
        self.births.push(self.births_this_tick as f32);
        self.deaths.push(self.deaths_this_tick as f32);
        self.avg_generation.push(avg_generation);

        self.births_this_tick = 0;
        self.deaths_this_tick = 0;
    }
}
