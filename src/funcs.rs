pub fn reduce_max_f64(acc: f64, el: f64) -> f64 {
    if acc > el {
        acc
    } else {
        el
    }
}

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
