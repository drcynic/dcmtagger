use crate::app_cmd::AppCmd;

#[derive(Default)]
pub struct EditHistory {
    undo_stack: Vec<Box<dyn AppCmd + Send + Sync>>,
    redo_stack: Vec<Box<dyn AppCmd + Send + Sync>>,
}

impl EditHistory {
    pub fn push(&mut self, change: Box<dyn AppCmd + Send + Sync>) {
        self.redo_stack.clear();
        self.undo_stack.push(change);
    }

    pub fn undo(&mut self) -> Option<&(dyn AppCmd + Send + Sync)> {
        let change = self.undo_stack.pop()?;
        self.redo_stack.push(change);
        self.redo_stack.last().map(|c| &**c)
    }

    pub fn redo(&mut self) -> Option<&(dyn AppCmd + Send + Sync)> {
        let change = self.redo_stack.pop()?;
        self.undo_stack.push(change);
        self.undo_stack.last().map(|c| &**c)
    }
}
