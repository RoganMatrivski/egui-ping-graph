use crate::series::Series;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use color_eyre::eyre::Report;

pub async fn run_cleaner(
    target: String,
    start_time: time::Instant,
    timeseries_hashref: Arc<Mutex<HashMap<String, Series>>>,
) -> Result<(), Report> {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
    let get_sec_elapsed = || (time::Instant::now() - start_time).as_seconds_f64();

    loop {
        if let Ok(mut ts_hash) = timeseries_hashref.lock() {
            if let Some(ts) = ts_hash.get_mut(&target) {
                ts.remove_older_than(get_sec_elapsed() - crate::MAX_HISTORY_SECS - 1.0);
                ts.update_pingstat();
            }
        }

        interval.tick().await;
    }
}
