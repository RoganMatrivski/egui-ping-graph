use crate::series::Series;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use color_eyre::eyre::{bail, Report};
use pinger::ping_with_interval;

pub fn run_pinger(
    target: String,
    start_time: time::Instant,
    timeseries_hashref: Arc<Mutex<HashMap<String, Series>>>,
    idxcolor: u8,
) -> Result<(), Report> {
    let get_sec_elapsed = || (time::Instant::now() - start_time).as_seconds_f64();

    let stream =
        ping_with_interval(target.clone(), std::time::Duration::from_millis(250), None).unwrap();

    while let Ok(pingres) = stream.recv() {
        if let Ok(mut ts_hash) = timeseries_hashref.lock() {
            use pinger::PingResult;

            if let Some(ts) = ts_hash.get_mut(&target) {
                ts.raw.push(match pingres {
                    PingResult::Pong(dur, _) => Some((get_sec_elapsed(), dur.as_secs_f64())),
                    PingResult::Timeout(_) => None,
                    PingResult::Unknown(_) => None,

                    PingResult::PingExited(e, _) if e.success() => {
                        break;
                    }
                    PingResult::PingExited(e, stderr) => {
                        bail!("There was an error running ping: {e}\nStderr: {stderr}\n");
                    }
                })
            } else {
                ts_hash.insert(target.clone(), Series::with_idxcolor(idxcolor));
            }
        }
    }

    Ok::<(), Report>(())
}
