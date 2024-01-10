// This is to disable console window
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use color_eyre::eyre::{bail, Report};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    vec,
};

mod funcs;
mod init;

const DEFAULT_HISTORY_SECS: f64 = 10.0;

// type Series = Vec<(f64, f64)>;
#[derive(Clone)]
pub struct Series {
    raw: Vec<Option<(f64, f64)>>,
    linecol: egui::Color32,
}

impl Default for Series {
    fn default() -> Self {
        let (r, g, b) = {
            use rand::Rng;

            let mut rng = rand::thread_rng();

            let hue = rng.gen_range(0.0..360.0);
            let sat = rng.gen_range(50.0..70.0);
            let lightness = rng.gen_range(65.0..75.0);

            funcs::hsl_to_rgb(hue, sat, lightness)
        };

        Self {
            raw: vec![],
            linecol: egui::Color32::from_rgb(r, g, b),
        }
    }
}

impl Series {
    fn get_splitted(&self) -> Vec<Vec<(f64, f64)>> {
        self.raw
            .split(|x| x.is_none())
            .map(|x| x.iter().map(|x| x.unwrap()).collect())
            .collect()
    }

    fn get_highest_value(&self) -> f64 {
        let reduce_vectuple =
            |x: &Vec<(f64, f64)>| x.iter().map(|x| x.1).fold(0.0, funcs::reduce_max_f64);

        self.get_splitted()
            .iter()
            .map(reduce_vectuple)
            .fold(0.0, funcs::reduce_max_f64)
    }

    fn splitted_to_plotpoints(&self) -> Vec<egui_plot::PlotPoints> {
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

    fn remove_older_than(&mut self, time: f64) {
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

pub struct App {
    t_since_start: time::Instant,
    timeseries_hash: Arc<Mutex<HashMap<String, Series>>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            timeseries_hash: Arc::new(Mutex::new(HashMap::new())),
            t_since_start: time::Instant::now(),
        }
    }
}

impl App {
    fn get_sec_since_start(&self) -> f64 {
        (time::Instant::now() - self.t_since_start).as_seconds_f64()
    }

    fn get_highest_value(&self) -> f64 {
        if let Ok(values) = self.timeseries_hash.clone().lock() {
            values
                .iter()
                .map(|x| x.1)
                .map(|x| x.get_highest_value())
                .max_by(|a, b| a.total_cmp(b))
                .unwrap_or(0.0)
        } else {
            0.0
        }
    }
}

#[tracing::instrument]
fn main() -> Result<(), Report> {
    init::initialize()?;

    let options = eframe::NativeOptions::default();
    let state = Box::<App>::default();

    let targets: Vec<_> = vec!["8.8.8.8", "8.8.4.4", "9.9.9.9"]
        .into_iter()
        .map(String::from)
        .collect();

    {
        use futures::future::select_all;
        use tokio::runtime::Runtime;

        let rt = Runtime::new()?;
        let _enter = rt.enter();
        let timeseries_hashclone = state.timeseries_hash.clone();

        std::thread::spawn(move || {
            rt.block_on(async {
                let curr_time = time::Instant::now();

                let pinger_handles: Vec<_> = targets
                    .clone()
                    .into_iter()
                    .map(|target| {
                        let timeseries_hashref = timeseries_hashclone.clone();

                        tokio::spawn(async move {
                            use pinger::ping_with_interval;

                            let target = target.clone();

                            let get_sec_elapsed =
                                || (time::Instant::now() - curr_time).as_seconds_f64();

                            let stream = ping_with_interval(
                                target.clone(),
                                std::time::Duration::from_millis(250),
                                None,
                            )
                            .unwrap();

                            while let Ok(pingres) = stream.recv() {
                                if let Ok(mut ts_hash) = timeseries_hashref.lock() {
                                    use pinger::PingResult;

                                    if let Some(ts) = ts_hash.get_mut(&target) {
                                        ts.raw.push(match pingres {
                                            PingResult::Pong(dur, _) => {
                                                Some((get_sec_elapsed(), dur.as_secs_f64()))
                                            }
                                            PingResult::Timeout(_) => None,
                                            PingResult::Unknown(_) => None,

                                            PingResult::PingExited(e, _) if e.success() => {
                                                break;
                                            }
                                            PingResult::PingExited(e, stderr) => {
                                                bail!(
                                        "There was an error running ping: {e}\nStderr: {stderr}\n"
                                    );
                                            }
                                        })
                                    } else {
                                        ts_hash.insert(target.clone(), Series::default());
                                    }
                                }
                            }

                            Ok::<(), Report>(())
                        })
                    })
                    .collect();

                let cleaner_handles: Vec<_> = targets
                    .clone()
                    .into_iter()
                    .map(|target| {
                        let timeseries_hashref = timeseries_hashclone.clone();

                        tokio::spawn(async move {
                            let target = target.clone();
                            let mut interval =
                                tokio::time::interval(tokio::time::Duration::from_secs(1));
                            let get_sec_elapsed =
                                || (time::Instant::now() - curr_time).as_seconds_f64();

                            loop {
                                if let Ok(mut ts_hash) = timeseries_hashref.lock() {
                                    if let Some(ts) = ts_hash.get_mut(&target) {
                                        ts.remove_older_than(
                                            get_sec_elapsed() - DEFAULT_HISTORY_SECS - 1.0,
                                        );
                                    }
                                }

                                interval.tick().await;
                            }
                        })
                    })
                    .collect();

                tokio::select! {
                    _ = select_all(pinger_handles) => {}
                    _ = select_all(cleaner_handles) => {}
                }
            });
        });
    }

    eframe::run_native("app_name", options, Box::new(|_ctx| state)).unwrap();

    Ok(())
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let f_elapsed = self.get_sec_since_start() / 1.0;

        egui::CentralPanel::default().show(ctx, |ui| {
            let plot = egui_plot::Plot::new("mesurment");
            plot.allow_boxed_zoom(false)
                .allow_double_click_reset(false)
                .allow_drag(false)
                .allow_scroll(false)
                .allow_zoom(false)
                .legend(egui_plot::Legend::default())
                .show(ui, |plot_ui| {
                    if let Ok(asdf) = self.timeseries_hash.lock() {
                        for (target, series) in asdf.iter() {
                            for points in series.splitted_to_plotpoints() {
                                plot_ui.line(
                                    egui_plot::Line::new(points)
                                        .color(series.linecol)
                                        .width(2.0)
                                        .name(target),
                                )
                            }
                        }
                    }

                    plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max(
                        [f_elapsed - DEFAULT_HISTORY_SECS, 0.0],
                        [f_elapsed, self.get_highest_value() + 0.05],
                    ));
                })
        });

        ctx.request_repaint();
    }
}
