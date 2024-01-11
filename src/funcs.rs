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
        let val = (val * 100.0).round() / 100.0;
        format!("{val} s")
    } else {
        let millis = (val * 1000.0 * 100.0).round() / 100.0;
        format!("{millis} ms")
    }
}

pub fn x_axis_fmt(val: f64) -> String {
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

pub fn y_axis_fmt(val: f64) -> String {
    fmt_float_s(val)
}

pub fn xy_label_fmt(serieslabel: &str, point: &egui_plot::PlotPoint) -> String {
    let latency_f = point.y;
    let time_f = point.x;

    let host_label = if !serieslabel.is_empty() {
        format!("Host: {serieslabel}\n")
    } else {
        "".to_string()
    };

    let lat_time = if time_f > 0.0 {
        format!(
            "Latency: {latency}\nTime: {t}",
            latency = y_axis_fmt(latency_f),
            t = x_axis_fmt(time_f),
        )
    } else {
        "".to_string()
    };

    host_label + &lat_time
}

pub fn pingstat_from_rawdata(rawdata: &[Option<(f64, f64)>]) -> crate::series::PingStatistics {
    use statrs::statistics::*;

    let data: Vec<f64> = rawdata.iter().filter_map(|x| x.map(|x| x.1)).collect();

    let timeouts = rawdata.iter().filter(|x| x.is_none()).count() as u32;

    let mut statdata = Data::new(data);

    crate::series::PingStatistics {
        last: *statdata.iter().last().unwrap_or(&0.0),
        min: statdata.min(),
        max: statdata.max(),
        avg: statdata.mean().unwrap_or(0.0),
        jitter: statdata.std_dev().unwrap_or(0.0),
        p95: statdata.percentile(95),
        timeouts,
    }
}
