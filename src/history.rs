use crate::model::Layout;
#[derive(Default)]
pub struct History {
    past: Vec<Layout>,
    future: Vec<Layout>,
}
impl History {
    pub fn record(&mut self, l: &Layout) {
        self.past.push(l.clone());
        if self.past.len() > 60 {
            self.past.remove(0);
        }
        self.future.clear();
    }
    pub fn undo(&mut self, l: &mut Layout) {
        if let Some(old) = self.past.pop() {
            self.future.push(std::mem::replace(l, old));
        }
    }
    pub fn redo(&mut self, l: &mut Layout) {
        if let Some(next) = self.future.pop() {
            self.past.push(std::mem::replace(l, next));
        }
    }
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
}
