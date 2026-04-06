use slotmap::DefaultKey;

use crate::dicom::TagElement;

#[derive(Debug, Clone)]
pub struct EditChange {
    pub node_id: DefaultKey,
    pub filename: String,
    pub old_element: TagElement,
    pub new_element: TagElement,
}

impl EditChange {
    pub fn new(node_id: DefaultKey, filename: String, old_element: TagElement, new_element: TagElement) -> Self {
        Self {
            node_id,
            filename,
            old_element,
            new_element,
        }
    }
}

#[derive(Debug, Default)]
pub struct EditHistory {
    undo_stack: Vec<EditChange>,
    redo_stack: Vec<EditChange>,
}

impl EditHistory {
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn push(&mut self, change: EditChange) {
        self.redo_stack.clear();
        self.undo_stack.push(change);
    }

    pub fn undo(&mut self) -> Option<&EditChange> {
        let change = self.undo_stack.pop()?;
        self.redo_stack.push(change);
        self.redo_stack.last()
    }

    pub fn redo(&mut self) -> Option<&EditChange> {
        let change = self.redo_stack.pop()?;
        self.undo_stack.push(change);
        self.undo_stack.last()
    }
}
