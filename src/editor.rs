use crate::{
    history::History,
    model::{Layout, Widget},
};
pub struct Editor {
    pub layout: Layout,
    pub selected: Option<String>,
    pub history: History,
    edit_start: Option<Layout>,
}
impl Editor {
    pub fn new(layout: Layout) -> Self {
        Self {
            layout,
            selected: None,
            history: History::default(),
            edit_start: None,
        }
    }
    pub fn selected_mut(&mut self) -> Option<&mut Widget> {
        let id = self.selected.as_ref()?;
        self.layout.widgets.iter_mut().find(|w| &w.id == id)
    }
    pub fn begin_edit(&mut self) {
        if self.edit_start.is_none() {
            self.edit_start = Some(self.layout.clone());
        }
    }
    pub fn end_edit(&mut self) {
        if let Some(before) = self.edit_start.take()
            && before != self.layout
        {
            self.history.record(&before);
        }
    }
    pub fn add(&mut self, kind: &str) {
        if self.layout.widgets.len() >= 100 {
            return;
        }
        self.end_edit();
        self.history.record(&self.layout);
        let w = Widget::new(kind, &self.layout);
        self.selected = Some(w.id.clone());
        self.layout.widgets.push(w);
    }
    pub fn paste_style(&mut self, source: &Widget) {
        self.begin_edit();
        if let Some(w) = self.selected_mut() {
            w.copy_style_from(source);
        }
        self.end_edit();
    }
    pub fn duplicate(&mut self) {
        self.end_edit();
        if self.layout.widgets.len() >= 100 {
            return;
        }
        if let Some(mut w) = self.selected_mut().cloned() {
            self.history.record(&self.layout);
            w.id = uuid::Uuid::new_v4().to_string();
            w.x += 24.;
            w.y += 24.;
            w.constrain(self.layout.width, self.layout.height);
            self.selected = Some(w.id.clone());
            self.layout.widgets.push(w);
        }
    }
    pub fn delete(&mut self) {
        self.end_edit();
        if let Some(id) = self.selected.take() {
            self.history.record(&self.layout);
            self.layout.widgets.retain(|w| w.id != id);
        }
    }
    pub fn nudge(&mut self, dx: f64, dy: f64) {
        let (width, height) = (self.layout.width, self.layout.height);
        if self.selected_mut().is_some_and(|w| !w.locked) {
            self.begin_edit();
            if let Some(w) = self.selected_mut() {
                w.x += dx;
                w.y += dy;
                w.constrain(width, height);
            }
            self.end_edit();
        }
    }
    pub fn nudge_size(&mut self, dw: f64, dh: f64) {
        let (width, height) = (self.layout.width, self.layout.height);
        if self.selected_mut().is_some_and(|w| !w.locked) {
            self.begin_edit();
            if let Some(w) = self.selected_mut() {
                w.w = (w.w + dw).min(width as f64 - w.x);
                w.h = (w.h + dh).min(height as f64 - w.y);
                w.constrain(width, height);
            }
            self.end_edit();
        }
    }
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        crate::model::validate_size(width, height)?;
        self.begin_edit();
        self.layout.resize(width, height)?;
        self.end_edit();
        Ok(())
    }
    pub fn layer(&mut self, up: bool) {
        self.end_edit();
        if let Some(i) = self
            .layout
            .widgets
            .iter()
            .position(|w| Some(&w.id) == self.selected.as_ref())
        {
            let j = if up {
                (i + 1).min(self.layout.widgets.len() - 1)
            } else {
                i.saturating_sub(1)
            };
            if i != j {
                self.history.record(&self.layout);
                self.layout.widgets.swap(i, j);
            }
        }
    }
    pub fn undo(&mut self) {
        self.end_edit();
        self.history.undo(&mut self.layout);
    }
    pub fn redo(&mut self) {
        self.end_edit();
        self.history.redo(&mut self.layout);
    }
    pub fn replace(&mut self, l: Layout) {
        self.end_edit();
        self.history.record(&self.layout);
        self.layout = l;
        self.selected = None;
    }
}
