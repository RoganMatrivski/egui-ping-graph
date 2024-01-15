use crate::{funcs, series::PingStatistics};

use std::time::{Duration, Instant};

use yata::core::Method;
use yata::methods::SMA;

pub struct Series {
    pub raw: Vec<Option<(Instant, u32)>>,
    pub stats: PingStatistics,

    appstart: Instant,

    filter: Box<dyn yata::core::Method<Params = u8, Input = f64, Output = f64>>,
    filtered: Vec<Option<(Instant, f64)>>,
}

impl Default for Series {
    fn default() -> Self {
        Self {
            raw: vec![],
            filtered: vec![],
            stats: Default::default(),

            appstart: Instant::now(),
            filter: Box::new(yata::methods::SMA::new(3, &0.0).unwrap()),
        }
    }
}

impl Series {
    pub fn get(&self, time: Option<Duration>) -> &[Option<(Instant, u32)>] {
        match time {
            None => &self.raw,
            Some(time) => {
                let split_pos = self.raw.iter().position(|x| {
                    if let Some((x, _)) = x {
                        *x > (self.appstart + time)
                    } else {
                        false
                    }
                });

                if let Some(pos) = split_pos {
                    self.raw.split_at(pos).1
                } else {
                    &self.raw
                }
            }
        }
    }

    pub fn get_splitted(&self, time: Option<Duration>) -> Vec<Vec<(Instant, u32)>> {
        self.get(time)
            .split(|x| x.is_none())
            .map(|x| x.iter().filter_map(|&y| y).collect())
            .collect()
    }

    pub fn get_valueonly(&self, time: Option<Duration>) -> Vec<u32> {
        self.get(time)
            .iter()
            .filter_map(|&x| x)
            .map(|(_, x)| x)
            .collect()
    }

    pub fn get_highest_value(&self, time: Option<Duration>) -> u32 {
        *self.get_valueonly(time).iter().max().unwrap_or(&0)
    }

    pub fn into_chunked_plotpoints(&self, time: Option<Duration>) -> Vec<egui_plot::PlotPoints> {
        use egui_plot::{PlotPoint, PlotPoints};

        self.get_splitted(time)
            .iter()
            .map(|x| {
                let plotiter = x
                    .iter()
                    .map(|&(x, y)| PlotPoint::from([(x - self.appstart).as_secs_f64(), y as f64]));

                PlotPoints::Owned(Vec::from_iter(plotiter))
            })
            .collect()
    }

    pub fn timeout_count(&self) -> usize {
        self.raw.iter().filter(|x| x.is_none()).count()
    }

    pub fn get_pingstat(&self, time: Option<Duration>) -> PingStatistics {
        use statrs::statistics::*;

        let data: Vec<_> = self.get_valueonly(time).iter().map(|&x| x as f64).collect();

        let timeouts = self.timeout_count() as u32;

        let mut statdata = Data::new(data);

        PingStatistics {
            last: *statdata.iter().last().unwrap_or(&0.0),
            min: statdata.min(),
            max: statdata.max(),
            avg: statdata.mean().unwrap_or(0.0),
            jitter: statdata.std_dev().unwrap_or(0.0),
            p95: statdata.percentile(95),
            timeouts,
        }
    }

    pub fn update_pingstat(&mut self, time: Option<Duration>) {
        self.stats = self.get_pingstat(time)
    }

    pub fn remove_olderthan(&mut self, time: Duration) {
        let split_pos = self.raw.iter().position(|x| {
            if let Some((x, _)) = x {
                *x > (self.appstart + time)
            } else {
                false
            }
        });

        if let Some(pos) = split_pos {
            self.raw = self.raw.split_at(pos).1.to_vec();
            self.filtered = self.filtered.split_at(pos).1.to_vec();
        }
    }

    pub fn push(&mut self, value: Option<(Instant, u32)>) {
        self.raw.push(value);

        self.filtered
            .push(value.map(|(t, v)| (t, self.filter.next(&(v as f64)))));
    }
}
