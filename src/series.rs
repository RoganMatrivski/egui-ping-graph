#[derive(Clone)]
pub struct Series {
    pub raw: Vec<Option<(f64, f64)>>,
    pub linecol: egui::Color32,
    pub linecol_idx: u8,
}

impl Default for Series {
    fn default() -> Self {
        Self {
            raw: vec![],
            linecol: egui::Color32::RED,
            linecol_idx: 0,
        }
    }
}

impl Series {
    pub fn with_hexcolor(hexcolor: &str) -> Self {
        Self {
            linecol: egui::Color32::from_hex(hexcolor).unwrap(), // TODO: Handle this conversion error
            ..Default::default()
        }
    }

    pub fn with_idxcolor(idx: u8) -> Self {
        Self {
            linecol_idx: idx,
            ..Default::default()
        }
    }

    pub fn get_splitted(&self) -> Vec<Vec<(f64, f64)>> {
        self.raw
            .split(|x| x.is_none())
            .map(|x| x.iter().map(|x| x.unwrap()).collect())
            .collect()
    }

    pub fn get_highest_value(&self) -> f64 {
        let reduce_vectuple = |x: &Vec<(f64, f64)>| {
            x.iter()
                .map(|x| x.1)
                .fold(0.0, crate::funcs::reduce_max_f64)
        };

        self.get_splitted()
            .iter()
            .map(reduce_vectuple)
            .fold(0.0, crate::funcs::reduce_max_f64)
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
