//! Pele xychart/db.ts ordered axes and plot model, 458322a5 (MIT, kepano).
use crate::compat::{source::space, svg::js_num, DiagramType};
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum XAxis {
    Band {
        title: String,
        categories: Vec<String>,
    },
    Linear {
        title: String,
        min: f64,
        max: f64,
    },
}
impl XAxis {
    pub fn title(&self) -> &str {
        match self {
            Self::Band { title, .. } | Self::Linear { title, .. } => title,
        }
    }
    fn set_title(&mut self, t: String) {
        match self {
            Self::Band { title, .. } | Self::Linear { title, .. } => *title = t,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct LinearAxis {
    #[serde(rename = "type")]
    pub axis_type: &'static str,
    pub title: String,
    pub min: f64,
    pub max: f64,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlotType {
    Line,
    Bar,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plot {
    #[serde(rename = "type")]
    pub plot_type: PlotType,
    pub title: String,
    pub data: Vec<(String, Option<f64>)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub point_labels: Option<Vec<String>>,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Vertical,
    Horizontal,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XyChartModel {
    #[serde(rename = "type")]
    pub diagram_type: DiagramType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<Orientation>,
    pub x_axis: XAxis,
    pub y_axis: LinearAxis,
    pub has_set_x_axis: bool,
    pub has_set_y_axis: bool,
    pub plots: Vec<Plot>,
}
impl XyChartModel {
    pub(super) fn new(title: Option<&str>) -> Self {
        Self {
            diagram_type: DiagramType::Xychart,
            title: title.map(str::to_owned),
            acc_title: None,
            acc_descr: None,
            orientation: None,
            x_axis: XAxis::Band {
                title: String::new(),
                categories: vec![],
            },
            y_axis: LinearAxis {
                axis_type: "linear",
                title: String::new(),
                min: f64::INFINITY,
                max: f64::NEG_INFINITY,
            },
            has_set_x_axis: false,
            has_set_y_axis: false,
            plots: vec![],
        }
    }
    pub(super) fn x_range(&mut self, min: f64, max: f64) {
        self.x_axis = XAxis::Linear {
            title: self.x_axis.title().into(),
            min,
            max,
        };
        self.has_set_x_axis = true;
    }
    pub(super) fn x_band(&mut self, categories: Vec<String>) {
        self.x_axis = XAxis::Band {
            title: self.x_axis.title().into(),
            categories: categories
                .into_iter()
                .map(|s| s.trim_matches(space).into())
                .collect(),
        };
        self.has_set_x_axis = true;
    }
    pub(super) fn y_range(&mut self, min: f64, max: f64) {
        self.y_axis = LinearAxis {
            axis_type: "linear",
            title: self.y_axis.title.clone(),
            min,
            max,
        };
        self.has_set_y_axis = true;
    }
    pub(super) fn x_title(&mut self, title: String) {
        self.x_axis.set_title(title.trim_matches(space).into())
    }
    pub(super) fn y_title(&mut self, title: String) {
        self.y_axis.title = title.trim_matches(space).into()
    }
    fn plot_data(&mut self, mut values: Vec<f64>) -> Vec<(String, Option<f64>)> {
        if values.is_empty() {
            return vec![];
        }
        if !self.has_set_x_axis {
            self.x_range(1., values.len() as f64)
        }
        if let XAxis::Band { categories, .. } = &self.x_axis {
            values.truncate(categories.len())
        }
        if !self.has_set_y_axis {
            let mut min = self.y_axis.min;
            let mut max = self.y_axis.max;
            for v in &values {
                if *v < min {
                    min = *v
                }
                if *v > max {
                    max = *v
                }
            }
            self.y_axis = LinearAxis {
                axis_type: "linear",
                title: self.y_axis.title.clone(),
                min,
                max,
            };
        }
        match &self.x_axis {
            XAxis::Band { categories, .. } => categories
                .iter()
                .enumerate()
                .map(|(i, c)| (c.clone(), values.get(i).copied()))
                .collect(),
            XAxis::Linear { min, max, .. } => {
                if values.len() == 1 {
                    return vec![(js_num(*min), Some(values[0]))];
                }
                let step = (max - min) / (values.len() - 1) as f64;
                values
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| (js_num(min + i as f64 * step), Some(v)))
                    .collect()
            }
        }
    }
    pub(super) fn add_plot(
        &mut self,
        plot_type: PlotType,
        title: String,
        points: Vec<(f64, String)>,
    ) {
        let data = self.plot_data(points.iter().map(|(v, _)| *v).collect());
        let point_labels = if plot_type == PlotType::Line
            && points
                .iter()
                .any(|(_, s)| !s.trim_matches(space).is_empty())
        {
            Some(
                points
                    .into_iter()
                    .map(|(_, s)| s.trim_matches(space).into())
                    .collect(),
            )
        } else {
            None
        };
        self.plots.push(Plot {
            plot_type,
            title: title.trim_matches(space).into(),
            data,
            point_labels,
        })
    }
}
