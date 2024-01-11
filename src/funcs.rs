use std::ops::RangeInclusive;

#[allow(dead_code)]
pub fn reduce_max_f64(acc: f64, el: f64) -> f64 {
    if acc > el {
        acc
    } else {
        el
    }
}

#[allow(dead_code)]
pub fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> (u8, u8, u8) {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_prime = hue / 60.0;
    let x = chroma * (1.0 - ((hue_prime % 2.0) - 1.0).abs());

    let (r, g, b) = match hue_prime {
        h if (0.0..1.0).contains(&h) => (chroma, x, 0.0),
        h if (1.0..2.0).contains(&h) => (x, chroma, 0.0),
        h if (2.0..3.0).contains(&h) => (0.0, chroma, x),
        h if (3.0..4.0).contains(&h) => (0.0, x, chroma),
        h if (4.0..5.0).contains(&h) => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };

    let m = lightness - 0.5 * chroma;

    (
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    )
}

pub fn fmt_float_s(val: f64) -> String {
    if val > 1.0 {
        format!("{val:.2} s")
    } else {
        let millis = val * 1000.0;
        format!("{millis:.2} ms")
    }
}

pub fn x_axis_fmt(val: f64, _charlen: usize, _curr_range: &RangeInclusive<f64>) -> String {
    if val.is_sign_negative() {
        return "".into();
    }

    let datetime_since_start = *crate::statics::DATETIME_START.get().unwrap();
    let asd = datetime_since_start + time::Duration::seconds_f64(val);

    format!(
        "{:0>2}:{:0>2}:{:0>2}",
        asd.hour(),
        asd.minute(),
        asd.second()
    )
}

pub fn y_axis_fmt(val: f64, _charlen: usize, _curr_range: &RangeInclusive<f64>) -> String {
    fmt_float_s(val)
}

pub fn mean(data: &[f64]) -> Option<f64> {
    let sum = data.iter().sum::<f64>();
    let count = data.len();

    match count {
        positive if positive > 0 => Some(sum / count as f64),
        _ => None,
    }
}

pub fn std_deviation(data: &[f64]) -> Option<f64> {
    match (mean(data), data.len()) {
        (Some(data_mean), count) if count > 0 => {
            let variance = data
                .iter()
                .map(|value| {
                    let diff = data_mean - *value;

                    diff * diff
                })
                .sum::<f64>()
                / count as f64;

            Some(variance.sqrt())
        }
        _ => None,
    }
}

pub fn pingstat_from_rawdata(rawdata: &[Option<(f64, f64)>]) -> crate::series::PingStatistics {
    let data: Vec<f64> = rawdata.iter().filter_map(|x| x.map(|x| x.1)).collect();

    let timeouts = rawdata.iter().filter(|x| x.is_none()).count() as u32;

    let p95 = {
        let mut sorted_data = data.clone();
        sorted_data.sort_by(|a, b| a.total_cmp(b));

        let percentile_index = ((95.0 / 100.0) * sorted_data.len() as f64) as usize;

        if percentile_index == 0 {
            0.0
        } else {
            sorted_data
                .get(percentile_index - 1)
                .cloned()
                .unwrap_or(0.0)
        }
    };

    crate::series::PingStatistics {
        last: *data.last().unwrap_or(&0.0),
        min: *data.iter().min_by(|a, b| a.total_cmp(b)).unwrap_or(&0.0),
        max: *data.iter().max_by(|a, b| a.total_cmp(b)).unwrap_or(&0.0),
        avg: mean(&data).unwrap_or(0.0),
        jitter: std_deviation(&data).unwrap_or(0.0),
        p95,
        timeouts,
    }
}
