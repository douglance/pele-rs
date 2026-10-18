//! Pele gantt/db.ts scheduling and ordered typed model, 458322a5 (MIT, kepano).
use super::dates::{utc_fields, Calendar, WEEKDAYS};
use crate::compat::{
    source::{space, url::safe_url},
    ErrorCode, PeleError, RenderResources,
};
use regex::Regex;
use serde::{ser::SerializeMap, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};
fn date_json<S: serde::Serializer>(date: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
    match date.and_then(utc_fields) {
        None => s.serialize_none(),
        Some(f) => {
            let c = f.civil;
            let year = if (0..=9999).contains(&c.year) {
                format!("{:04}", c.year)
            } else {
                format!("{}{:06}", if c.year < 0 { '-' } else { '+' }, c.year.abs())
            };
            s.serialize_str(&format!(
                "{year}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                c.month, c.day, c.hour, c.minute, c.second, c.millisecond
            ))
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GanttTask {
    pub id: String,
    pub task: String,
    pub section: String,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "date_json")]
    pub start_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "date_json")]
    pub end_time: Option<f64>,
    #[serde(serialize_with = "date_json")]
    pub render_end_time: Option<f64>,
    pub manual_end_time: bool,
    pub processed: bool,
    pub active: bool,
    pub done: bool,
    pub crit: bool,
    pub milestone: bool,
    pub vert: bool,
    pub order: isize,
    pub classes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_task_id: Option<String>,
    pub raw: TaskRaw,
}
#[derive(Clone, Debug, Serialize)]
pub struct TaskRaw {
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    pub end: String,
}
#[derive(Clone, Debug)]
pub struct GanttModel {
    task_at: HashMap<String, usize>,
    by_at: HashMap<String, usize>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub date_format: String,
    pub axis_format: String,
    pub tick_interval: Option<String>,
    pub today_marker: String,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    pub inclusive_end_dates: bool,
    pub top_axis: bool,
    pub display_mode: String,
    pub weekday: Option<String>,
    pub weekend: String,
    pub sections: Vec<String>,
    pub links: Vec<(String, String)>,
    pub tasks: Vec<GanttTask>,
    pub(crate) by_id: Vec<(String, usize)>,
    current_section: String,
    task_count: usize,
    last_order: usize,
    last_task_id: Option<String>,
    pub(crate) now: f64,
    pub(crate) work: f64,
    include_set: HashSet<String>,
    exclude_set: HashSet<String>,
    excluded_days: Vec<bool>,
    dated_tokens: bool,
}
impl Serialize for GanttModel {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("type", "gantt")?;
        for (key, value) in [
            ("title", &self.title),
            ("accTitle", &self.acc_title),
            ("accDescr", &self.acc_descr),
        ] {
            if let Some(v) = value {
                m.serialize_entry(key, v)?
            }
        }
        m.serialize_entry("dateFormat", &self.date_format)?;
        m.serialize_entry("axisFormat", &self.axis_format)?;
        if let Some(v) = &self.tick_interval {
            m.serialize_entry("tickInterval", v)?
        }
        m.serialize_entry("todayMarker", &self.today_marker)?;
        m.serialize_entry("includes", &self.includes)?;
        m.serialize_entry("excludes", &self.excludes)?;
        m.serialize_entry("inclusiveEndDates", &self.inclusive_end_dates)?;
        m.serialize_entry("topAxis", &self.top_axis)?;
        m.serialize_entry("displayMode", &self.display_mode)?;
        if let Some(v) = &self.weekday {
            m.serialize_entry("weekday", v)?
        }
        m.serialize_entry("weekend", &self.weekend)?;
        m.serialize_entry("sections", &self.sections)?;
        m.serialize_entry("links", &self.links)?;
        m.serialize_entry("tasks", &self.tasks)?;
        m.serialize_entry(
            "byId",
            &self
                .by_id
                .iter()
                .map(|(id, i)| (id, &self.tasks[*i]))
                .collect::<Vec<_>>(),
        )?;
        m.serialize_entry("currentSection", &self.current_section)?;
        m.serialize_entry("taskCount", &self.task_count)?;
        m.serialize_entry("lastOrder", &self.last_order)?;
        if let Some(v) = &self.last_task_id {
            m.serialize_entry("lastTaskId", v)?
        }
        m.serialize_entry("options", &serde_json::json!({}))?;
        m.serialize_entry("now", &self.now)?;
        m.serialize_entry("work", &self.work)?;
        m.serialize_entry("includeSet", &serde_json::json!({}))?;
        m.serialize_entry("excludeSet", &serde_json::json!({}))?;
        m.serialize_entry("excludedDays", &self.excluded_days)?;
        m.serialize_entry("datedTokens", &self.dated_tokens)?;
        m.end()
    }
}
pub fn error(message: impl Into<String>, code: ErrorCode) -> PeleError {
    let mut e = PeleError::new(message, code);
    e.diagram_type = Some("gantt".into());
    e
}
fn patterns() -> &'static Vec<Regex> {
    static R: OnceLock<Vec<Regex>> = OnceLock::new();
    R.get_or_init(|| {
        [
            r"^[YMDHhmsSZXx\-/.:, T]*$",
            r"^\s*(active|done|crit|milestone|vert)\s*$",
            r"^after\s+([0-9A-Za-z_ -]+)",
            r"^until\s+([0-9A-Za-z_ -]+)",
            r"^([0-9]+(?:\.[0-9]+)?)([Mdhmswy]|ms)$",
            r"[\s,]+",
            r"^[0-9]+$",
        ]
        .iter()
        .map(|s| compiled_regex(s).unwrap())
        .collect()
    })
}
impl GanttModel {
    pub fn new(title: Option<&str>, display_mode: String) -> Self {
        Self {
            task_at: HashMap::new(),
            by_at: HashMap::new(),
            title: title.map(str::to_string),
            acc_title: None,
            acc_descr: None,
            date_format: String::new(),
            axis_format: String::new(),
            tick_interval: None,
            today_marker: String::new(),
            includes: vec![],
            excludes: vec![],
            inclusive_end_dates: false,
            top_axis: false,
            display_mode,
            weekday: None,
            weekend: "saturday".into(),
            sections: vec![],
            links: vec![],
            tasks: vec![],
            by_id: vec![],
            current_section: String::new(),
            task_count: 0,
            last_order: 0,
            last_task_id: None,
            now: 0.,
            work: 0.,
            include_set: HashSet::new(),
            exclude_set: HashSet::new(),
            excluded_days: vec![],
            dated_tokens: false,
        }
    }
    pub fn tokens(existing: &mut Vec<String>, text: &str) {
        for t in patterns()[5]
            .split(&text.to_lowercase())
            .filter(|s| !s.is_empty())
        {
            if !existing.iter().any(|s| s == t) {
                existing.push(t.into())
            }
        }
    }
    pub fn add_section(&mut self, text: String) {
        self.current_section = text.clone();
        self.sections.push(text)
    }
    pub fn add_task(&mut self, descr: String, data: String) -> Result<(), PeleError> {
        let mut fields: Vec<&str> = data.strip_prefix(':').unwrap_or(&data).split(',').collect();
        let mut task = GanttTask {
            id: String::new(),
            task: descr,
            section: self.current_section.clone(),
            start_time: None,
            end_time: None,
            render_end_time: None,
            manual_end_time: false,
            processed: false,
            active: false,
            done: false,
            crit: false,
            milestone: false,
            vert: false,
            order: -1,
            classes: vec![],
            prev_task_id: self.last_task_id.clone(),
            raw: TaskRaw {
                data: data.clone(),
                start: None,
                end: String::new(),
            },
        };
        while let Some(tag) = fields.first().and_then(|s| patterns()[1].captures(s)) {
            match &tag[1] {
                "active" => task.active = true,
                "done" => task.done = true,
                "crit" => task.crit = true,
                "milestone" => task.milestone = true,
                "vert" => task.vert = true,
                _ => {}
            }
            fields.remove(0);
        }
        if fields.is_empty() {
            return Err(error(
                format!(
                    "Task \"{}\" needs a duration or an end date.",
                    task.task.trim_matches(space)
                ),
                ErrorCode::Semantic,
            ));
        }
        if fields.len() > 3 {
            return Err(error(format!("Task \"{}\" has too many fields. A task takes an id, a start, and an end or duration.",task.task.trim_matches(space)),ErrorCode::Semantic));
        }
        let fields: Vec<_> = fields.iter().map(|s| s.trim_matches(space)).collect();
        task.id = if fields.len() == 3 {
            fields[0].into()
        } else {
            self.task_count += 1;
            format!("task{}", self.task_count)
        };
        if fields.len() > 1 {
            task.raw.start = Some(fields[fields.len() - 2].into())
        }
        task.raw.end = fields.last().unwrap().to_string();
        if !task.vert {
            task.order = self.last_order as isize;
            self.last_order += 1
        }
        self.last_task_id = Some(task.id.clone());
        if let Some(&at) = self.by_at.get(&task.id) {
            self.by_id[at].1 = self.tasks.len()
        } else {
            self.by_at.insert(task.id.clone(), self.by_id.len());
            self.by_id.push((task.id.clone(), self.tasks.len()))
        }
        self.task_at.insert(task.id.clone(), self.tasks.len());
        self.tasks.push(task);
        Ok(())
    }
    pub fn class(&mut self, ids: &str) {
        for id in ids.split(',') {
            if let Some(i) = self.task_at.get(id) {
                self.tasks[*i].classes.push("clickable".into())
            }
        }
    }
    pub fn link(&mut self, ids: &str, url: &str) -> Result<(), PeleError> {
        let href = safe_url(url)?;
        for id in ids.split(',') {
            if self.task_at.contains_key(id) {
                if let Some((_, v)) = self.links.iter_mut().find(|(k, _)| k == id) {
                    *v = href.clone()
                } else {
                    self.links.push((id.into(), href.clone()))
                }
            }
        }
        self.class(ids);
        Ok(())
    }
    /// Schedule a reusable parsed document against the host calendar and clock.
    pub fn get_tasks(
        &self,
        now: Option<f64>,
        resources: &mut dyn RenderResources,
    ) -> Result<Vec<GanttTask>, PeleError> {
        let now = if let Some(now) = now.filter(|n| n.is_finite()) {
            now
        } else {
            resources.now_millis()?
        };
        Ok(self.scheduled(now, resources)?.tasks)
    }
    pub fn scheduled(
        &self,
        now: f64,
        resources: &mut dyn RenderResources,
    ) -> Result<Self, PeleError> {
        let now = if now.is_finite() {
            now
        } else {
            resources.now_millis()?
        };
        let mut m = self.clone();
        let mut cal = Calendar::new(resources);
        m.schedule(now, &mut cal)?;
        if let Some(e) = cal.error {
            return Err(e);
        }
        Ok(m)
    }
    pub(crate) fn schedule(&mut self, now: f64, cal: &mut Calendar<'_>) -> Result<(), PeleError> {
        self.now = now;
        self.work = 0.;
        self.include_set = self.includes.iter().cloned().collect();
        self.exclude_set = self.excludes.iter().cloned().collect();
        let start = if self.weekend == "friday" { 5 } else { 6 };
        let weekends = self.exclude_set.contains("weekends");
        self.excluded_days = WEEKDAYS
            .iter()
            .enumerate()
            .map(|(day, name)| {
                let iso = if day == 0 { 7 } else { day };
                weekends && (iso == start || iso == start + 1)
                    || self.exclude_set.contains(&name.to_lowercase())
            })
            .collect();
        self.dated_tokens = !self.includes.is_empty()
            || !patterns()[0].is_match(self.date_format.trim_matches(space));
        for token in &self.excludes {
            if token != "weekends" && !WEEKDAYS.iter().any(|d| d.to_lowercase() == *token) {
                self.dated_tokens = true
            }
        }
        for t in &mut self.tasks {
            t.start_time = None;
            t.end_time = None;
            t.render_end_time = None;
            t.processed = false;
            t.manual_end_time = false
        }
        for _ in 0..11 {
            let mut all = true;
            for i in 0..self.tasks.len() {
                self.compile_task(i, cal)?;
                all &= self.tasks[i].processed
            }
            if all {
                break;
            }
        }
        Ok(())
    }
    fn spend(&mut self, units: f64) -> Result<(), PeleError> {
        self.work += units;
        if self.work > 4e8 {
            Err(error(
                "The gantt chart takes too much work to schedule.",
                ErrorCode::Limit,
            ))
        } else {
            Ok(())
        }
    }
    fn parse(
        &mut self,
        s: &str,
        template: &str,
        cal: &mut Calendar<'_>,
    ) -> Result<Option<f64>, PeleError> {
        self.spend(
            (template.encode_utf16().count() + 1) as f64 * (s.encode_utf16().count() + 1) as f64,
        )?;
        Ok(cal.parse_strict(s, template, self.now))
    }
    pub(crate) fn invalid(&mut self, date: f64, cal: &mut Calendar<'_>) -> Result<bool, PeleError> {
        let Some(f) = cal.fields(date) else {
            return Ok(false);
        };
        let weekday = self
            .excluded_days
            .get(f.weekday as usize)
            .copied()
            .unwrap_or(false);
        if !self.dated_tokens {
            return Ok(weekday);
        }
        let template = self.date_format.trim_matches(space).to_string();
        let date_only = cal.iso(date);
        self.spend(200.)?;
        let formatted = if template != "YYYY-MM-DD" {
            self.spend(400. + template.encode_utf16().count() as f64 * 8.)?;
            cal.format(date, &template)
        } else {
            Some(date_only.clone())
        };
        let listed = |s: &HashSet<String>| {
            s.contains(&date_only) || formatted.as_ref().is_some_and(|f| s.contains(f))
        };
        if listed(&self.include_set) {
            return Ok(false);
        }
        Ok(weekday || listed(&self.exclude_set))
    }
    fn refs(&self, list: &str) -> Vec<usize> {
        list.split(' ')
            .filter(|s| !s.is_empty())
            .filter_map(|id| self.task_at.get(id).copied())
            .collect()
    }
    fn get_start(&mut self, raw: &str, cal: &mut Calendar<'_>) -> Result<Option<f64>, PeleError> {
        let s = raw.trim_matches(space);
        let template = self.date_format.trim_matches(space).to_string();
        if matches!(template.as_str(), "x" | "X") && patterns()[6].is_match(s) {
            let t = super::dates::clip(s.parse().unwrap_or(f64::NAN));
            return if t.is_finite() {
                Ok(Some(t))
            } else {
                Err(error(format!("Invalid date:{s}"), ErrorCode::Semantic))
            };
        }
        if let Some(cap) = patterns()[2].captures(s) {
            let mut latest: Option<usize> = None;
            for i in self.refs(&cap[1]) {
                if latest.is_none()
                    || self.tasks[i]
                        .end_time
                        .zip(self.tasks[latest.unwrap()].end_time)
                        .is_some_and(|(a, b)| a > b)
                {
                    latest = Some(i)
                }
            }
            return Ok(latest.map_or_else(|| Some(cal.today(self.now)), |i| self.tasks[i].end_time));
        }
        if let Some(t) = self.parse(s, &template, cal)? {
            return Ok(Some(t));
        }
        let date = cal.resources.parse_date(s)?.unwrap_or(f64::NAN);
        if !date.is_finite()
            || cal
                .fields(date)
                .is_none_or(|f| !(-10000..=10000).contains(&f.civil.year))
        {
            return Err(error(format!("Invalid date:{s}"), ErrorCode::Semantic));
        }
        Ok(Some(date))
    }
    fn get_end(
        &mut self,
        start: f64,
        raw: &str,
        cal: &mut Calendar<'_>,
    ) -> Result<Option<f64>, PeleError> {
        let s = raw.trim_matches(space);
        if let Some(cap) = patterns()[3].captures(s) {
            let mut earliest: Option<usize> = None;
            for i in self.refs(&cap[1]) {
                if earliest.is_none()
                    || self.tasks[i]
                        .start_time
                        .zip(self.tasks[earliest.unwrap()].start_time)
                        .is_some_and(|(a, b)| a < b)
                {
                    earliest = Some(i)
                }
            }
            return Ok(
                earliest.map_or_else(|| Some(cal.today(self.now)), |i| self.tasks[i].start_time)
            );
        }
        let template = self.date_format.trim_matches(space).to_string();
        if let Some(t) = self.parse(s, &template, cal)? {
            return Ok(Some(if self.inclusive_end_dates {
                cal.add_days(t, 1.)
            } else {
                t
            }));
        }
        let Some(cap) = patterns()[4].captures(s) else {
            return Ok(Some(start));
        };
        let n = cap[1].parse::<f64>().unwrap_or(f64::NAN);
        let t = cal.add(start, n, &cap[2]);
        Ok(Some(if t.is_finite() { t } else { start }))
    }
    fn compile_task(&mut self, i: usize, cal: &mut Calendar<'_>) -> Result<(), PeleError> {
        let raw = self.tasks[i].raw.clone();
        if let Some(start) = raw.start {
            if let Some(t) = self.get_start(&start, cal)? {
                self.tasks[i].start_time = Some(t)
            }
        } else {
            self.tasks[i].start_time = if let Some(p) = self.tasks[i]
                .prev_task_id
                .as_ref()
                .and_then(|id| self.task_at.get(id))
            {
                self.tasks[*p].end_time
            } else {
                Some(cal.today(self.now))
            }
        }
        let Some(start) = self.tasks[i].start_time else {
            return Ok(());
        };
        let Some(end) = self.get_end(start, &raw.end, cal)? else {
            return Ok(());
        };
        self.tasks[i].end_time = Some(end);
        self.tasks[i].processed = true;
        self.tasks[i].manual_end_time = self.parse(&raw.end, "YYYY-MM-DD", cal)?.is_some();
        if self.excludes.is_empty() || self.tasks[i].manual_end_time {
            return Ok(());
        }
        let mut cursor = cal.add_days(start, 1.);
        let mut end = end;
        let max = cal.add_days(end, 10000.);
        let mut invalid = false;
        let mut render_end = None;
        while cursor <= end {
            self.spend(400.)?;
            if !invalid {
                render_end = Some(end)
            }
            invalid = self.invalid(cursor, cal)?;
            if invalid {
                end = cal.add_days(end, 1.);
                if end > max {
                    return Err(error("Failed to find a valid date that was not excluded by `excludes` after 10,000 iterations.",ErrorCode::Semantic));
                }
            }
            cursor = cal.add_days(cursor, 1.)
        }
        self.tasks[i].end_time = Some(end);
        self.tasks[i].render_end_time = render_end;
        Ok(())
    }
}

fn compiled_regex(pattern: &str) -> Result<Regex, regex::Error> {
    Regex::new(&pattern.replace(r"\s", crate::compat::text::SPACE))
}
