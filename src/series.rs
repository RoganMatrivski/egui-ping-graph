use crate::funcs;

#[derive(Clone)]
pub struct Series {
    pub raw: Vec<Option<(f64, f64)>>,
    pub linecol: egui::Color32,
    pub linecol_idx: u8,
    pub stats: PingStatistics,
}

#[derive(Clone, Default)]
pub struct PingStatistics {
    pub last: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub jitter: f64,
    pub p95: f64,
    pub timeouts: u32,
}

impl Default for Series {
    fn default() -> Self {
        Self {
            raw: vec![],
            linecol: egui::Color32::RED,
            linecol_idx: 0,

            stats: Default::default(),
        }
    }
}

// #[allow(dead_code)]
impl Series {
    pub fn with_idxcolor(idx: u8) -> Self {
        Self {
            linecol_idx: idx,
            ..Default::default()
        }
    }

    pub fn get_younger_than(&self, time: f64) -> &[Option<(f64, f64)>] {
        let split_pos = self.raw.iter().position(|x| {
            if let Some((x, _)) = x {
                *x > time
            } else {
                false
            }
        });

        if let Some(pos) = split_pos {
            self.raw.split_at(pos).1
        } else {
            self.raw.as_slice()
        }
    }

    pub fn get_splitted(&self) -> Vec<Vec<(f64, f64)>> {
        self.raw
            .split(|x| x.is_none())
            .map(|x| x.iter().map(|x| x.unwrap()).collect())
            .collect()
    }

    pub fn get_highest_value(&self) -> f64 {
        self.raw
            .iter()
            .filter_map(|x| x.map(|x| x.1))
            .max_by(|a, b| a.total_cmp(b))
            .unwrap()
    }

    pub fn get_highest_value_youngerthan(&self, time: f64) -> f64 {
        self.get_younger_than(time)
            .iter()
            .filter_map(|x| x.map(|x| x.1))
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(0.0)
    }

    pub fn get_pingstat(&self) -> PingStatistics {
        funcs::pingstat_from_rawdata(&self.raw)
    }

    pub fn update_pingstat(&mut self) {
        self.stats = self.get_pingstat();
    }

    pub fn splitted_to_plotpoints(&self) -> Vec<egui_plot::PlotPoints> {
        self.get_splitted()
            .into_iter()
            .map(|x| {
                egui_plot::PlotPoints::Owned(Vec::from_iter(
                    x.iter()
                        .map(|&(x, y)| egui_plot::PlotPoint::from([x, y]))
                        .collect::<Vec<_>>(),
                ))
            })
            .collect()
    }

    pub fn remove_older_than(&mut self, time: f64) {
        let split_pos = self.raw.iter().position(|x| {
            if let Some((x, _)) = x {
                *x > time
            } else {
                false
            }
        });

        if let Some(pos) = split_pos {
            self.raw = self.raw.split_at(pos).1.to_vec();
        }
    }
}
