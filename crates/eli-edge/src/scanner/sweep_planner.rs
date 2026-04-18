#[derive(Debug, Clone, Copy)]
pub struct SweepPoint {
    pub center_hz: f64,
    pub lower_edge_hz: f64,
    pub upper_edge_hz: f64,
    pub priority: f32,
}

#[derive(Debug, Clone)]
pub struct SweepPlannerConfig {
    pub start_hz: f64,
    pub end_hz: f64,
    pub sample_rate_hz: f64,
    pub usable_bandwidth_hz: f64,
    pub overlap_fraction: f64,
}

impl SweepPlannerConfig {
    pub fn step_hz(&self) -> f64 {
        self.usable_bandwidth_hz * (1.0 - self.overlap_fraction)
    }
}

pub struct SweepPlanner {
    points: Vec<SweepPoint>,
    usable_bandwidth_hz: f64,
}

impl SweepPlanner {
    pub fn new_linear(config: SweepPlannerConfig) -> Self {
        let mut points = Vec::new();
        let step_hz = config.step_hz();
        let half_span_hz = config.sample_rate_hz / 2.0;
        let mut center_hz = config.start_hz;

        while center_hz <= config.end_hz {
            points.push(SweepPoint {
                center_hz,
                lower_edge_hz: center_hz - half_span_hz,
                upper_edge_hz: center_hz + half_span_hz,
                priority: 1.0,
            });
            center_hz += step_hz;
        }

        Self {
            points,
            usable_bandwidth_hz: config.usable_bandwidth_hz,
        }
    }

    pub fn new_priority(config: SweepPlannerConfig, hotspots: &[(f64, f32)]) -> Self {
        let mut planner = Self::new_linear(config);
        let usable_bandwidth_hz = planner.usable_bandwidth_hz;

        for point in &mut planner.points {
            for (hotspot_hz, boost) in hotspots {
                let distance_hz = (point.center_hz - *hotspot_hz).abs();

                if distance_hz <= usable_bandwidth_hz / 2.0 {
                    point.priority += *boost;
                }
            }
        }

        planner.sort_by_priority();
        planner
    }

    pub fn points(&self) -> &[SweepPoint] {
        &self.points
    }

    pub fn pop_next(&mut self) -> Option<SweepPoint> {
        if self.points.is_empty() {
            return None;
        }

        Some(self.points.remove(0))
    }

    pub fn reprioritize_near(&mut self, freq_hz: f64, boost: f32, radius_hz: f64) {
        for point in &mut self.points {
            let distance_hz = (point.center_hz - freq_hz).abs();

            if distance_hz <= radius_hz {
                point.priority += boost;
            }
        }

        self.sort_by_priority();
    }

    fn sort_by_priority(&mut self) {
        self.points.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}
