// This is to disable console window
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use color_eyre::eyre::Report;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    vec,
};

use crate::series::Series;

mod app_cleaner;
mod app_pinger;
mod funcs;
mod init;
mod series;
mod statics;

// const DEFAULT_HISTORY_SECS: f64 = 10.0;
const DEFAULT_OFFSET: f64 = 1.0;
const MAX_HISTORY_SECS: f64 = 120.0;

const MAX_FPS: u32 = 30;
const MAX_FRAME_SLEEP: f64 = 1000.0 / MAX_FPS as f64;
const MAX_FRAME_SLEEP_DUR: std::time::Duration =
    std::time::Duration::from_millis(MAX_FRAME_SLEEP as u64);

pub struct App {
    // t_since_start: time::Instant,
    // datetime_since_start: time::OffsetDateTime,
    timeseries_hash: Arc<Mutex<HashMap<String, Series>>>,
    color_preset: Vec<egui::Color32>,
    color_preset_shfidx: Vec<u8>,

    history_window: f64,

    top_padding: f64,
}

impl Default for App {
    fn default() -> Self {
        use rand::seq::SliceRandom;

        let color_preset: Vec<_> = ["#48B7B2", "#7A48B7", "#B7484D", "#85B748"]
            .iter()
            .map(|&x| egui::Color32::from_hex(x).unwrap())
            .collect();

        let mut rng = rand::thread_rng();
        let mut color_preset_shfidx: Vec<_> = (0u8..color_preset.len() as u8).collect();
        color_preset_shfidx.shuffle(&mut rng);

        Self {
            color_preset,
            color_preset_shfidx,

            timeseries_hash: Arc::new(Mutex::new(HashMap::new())),
            // t_since_start: time::Instant::now(),
            // datetime_since_start: time::OffsetDateTime::now_utc(),
            history_window: 10.0,

            top_padding: 0.01,
        }
    }
}

#[allow(dead_code)]
impl App {
    fn get_sec_since_start(&self) -> f64 {
        (time::Instant::now() - *statics::I_START.get().unwrap()).as_seconds_f64()
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

    fn get_highest_value_youngerthan(&self, time: f64) -> f64 {
        if let Ok(values) = self.timeseries_hash.clone().lock() {
            values
                .iter()
                .map(|x| x.1)
                .map(|x| x.get_highest_value_youngerthan(time))
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

    #[cfg(debug_assertions)]
    {
        println!("issa debug'a");

        puffin::set_scopes_on(true); // tell puffin to collect data

        match puffin_http::Server::new("127.0.0.1:8585") {
            Ok(puffin_server) => {
                eprintln!(
                    "Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585"
                );

                std::process::Command::new("puffin_viewer")
                    .arg("--url")
                    .arg("127.0.0.1:8585")
                    .spawn()
                    .ok();

                // We can store the server if we want, but in this case we just want
                // it to keep running. Dropping it closes the server, so let's not drop it!
                #[allow(clippy::mem_forget)]
                std::mem::forget(puffin_server);
            }
            Err(err) => {
                eprintln!("Failed to start puffin server: {err}");
            }
        };
    }

    {
        // OnceCell statics init

        statics::DATETIME_START.get_or_init(time::OffsetDateTime::now_utc);
        statics::I_START.get_or_init(time::Instant::now);
    }

    let options = eframe::NativeOptions::default();
    let state = Box::<App>::default();

    let targets: Vec<_> = vec!["8.8.8.8", "9.9.9.9", "1.1.1.1"]
        .into_iter()
        .map(String::from)
        .collect();

    {
        use futures::future::select_all;
        use tokio::runtime::Runtime;

        let rt = Runtime::new()?;
        let _enter = rt.enter();
        let timeseries_hashclone = state.timeseries_hash.clone();
        let available_idxcolor = state.color_preset_shfidx.clone();

        std::thread::spawn(move || {
            rt.block_on(async {
                let curr_time = time::Instant::now();
                let mut available_idxcolor = available_idxcolor;

                let pinger_handles: Vec<_> = targets
                    .clone()
                    .into_iter()
                    .map(|target| {
                        let timeseries_hashref = timeseries_hashclone.clone();
                        let idxcolor = available_idxcolor.pop().unwrap_or(0);

                        tokio::spawn(async move {
                            app_pinger::run_pinger(
                                target.clone(),
                                curr_time,
                                timeseries_hashref,
                                idxcolor,
                            )
                        })
                    })
                    .collect();

                let cleaner_handles: Vec<_> = targets
                    .clone()
                    .into_iter()
                    .map(|target| {
                        let timeseries_hashref = timeseries_hashclone.clone();

                        tokio::spawn(async move {
                            app_cleaner::run_cleaner(target.clone(), curr_time, timeseries_hashref)
                                .await
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
        puffin::profile_scope!("update");
        let frame_istart = std::time::Instant::now();
        let f_elapsed = self.get_sec_since_start() / 1.0;
        let start_time = *statics::I_START.get().unwrap();
        let highest_value = self.get_highest_value_youngerthan(
            (frame_istart - start_time).as_seconds_f64() - self.history_window,
        );

        ctx.set_visuals(egui::Visuals::dark());

        egui::CentralPanel::default().show(ctx, |ui| {
            puffin::profile_scope!("CentralPanel_draw");

            egui::CollapsingHeader::new("Options")
                .default_open(false)
                .show_unindented(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Window size");
                        ui.add(egui::Slider::new(
                            &mut self.history_window,
                            5.0..=MAX_HISTORY_SECS,
                        ));
                    });

                    egui::CollapsingHeader::new("Extra Options")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Top Padding");
                                ui.add(egui::Slider::new(&mut self.top_padding, 0.05..=2.0));
                            })
                        });
                });

            if let Ok(asdf) = self.timeseries_hash.lock() {
                egui::CollapsingHeader::new("Details")
                    .default_open(true)
                    .show_unindented(ui, |ui| {
                        egui::Grid::new("pingmeasurements").show(ui, |ui| {
                            for (target, series) in asdf.iter() {
                                let stats = &series.stats;

                                ui.label(target);

                                ui.label(format!("last: {}", funcs::fmt_float_s(stats.last)));
                                ui.label(format!("min: {}", funcs::fmt_float_s(stats.min)));
                                ui.label(format!("max: {}", funcs::fmt_float_s(stats.max)));
                                ui.label(format!("avg: {}", funcs::fmt_float_s(stats.avg)));
                                ui.label(format!("jtr: {}", funcs::fmt_float_s(stats.jitter)));
                                ui.label(format!("p95: {}", funcs::fmt_float_s(stats.p95)));
                                ui.label(format!("t/0: {}", stats.timeouts));
                                ui.end_row();
                            }
                        })
                    });
            }

            let plot = egui_plot::Plot::new("mesurment");
            plot.allow_boxed_zoom(false)
                .allow_double_click_reset(false)
                .allow_drag(false)
                .allow_scroll(false)
                .allow_zoom(false)
                .legend(egui_plot::Legend::default())
                .x_axis_formatter(funcs::x_axis_fmt)
                .y_axis_formatter(funcs::y_axis_fmt)
                .x_axis_label("Time")
                .y_axis_label("Latency")
                .show(ui, |plot_ui| {
                    puffin::profile_scope!("Plot_draw");

                    if let Ok(asdf) = self.timeseries_hash.lock() {
                        for (target, series) in asdf.iter() {
                            puffin::profile_scope!("series_iter", target);
                            for points in series.splitted_to_plotpoints() {
                                puffin::profile_scope!("lines_iter");
                                plot_ui.line(
                                    egui_plot::Line::new(points)
                                        .color(self.color_preset[series.linecol_idx as usize])
                                        .width(3.0)
                                        .fill(0.0)
                                        .name(target),
                                )
                            }
                        }
                    }

                    plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max(
                        [f_elapsed - DEFAULT_OFFSET - self.history_window, 0.0],
                        [f_elapsed - DEFAULT_OFFSET, highest_value + self.top_padding],
                    ));
                })
        });

        {
            puffin::profile_scope!("fpslimit_sleep");

            let threadsleep_dur = MAX_FRAME_SLEEP_DUR - frame_istart.elapsed();
            std::thread::sleep(threadsleep_dur);
        }

        ctx.request_repaint();
        puffin::GlobalProfiler::lock().new_frame();
    }
}
