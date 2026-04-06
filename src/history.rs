use slotmap::DefaultKey;

use crate::dicom::TagElement;

pub trait AppCmd {
    fn apply(&self, app: &mut crate::app::App);
    fn undo(&self, app: &mut crate::app::App);
}

#[derive(Debug, Clone)]
pub struct TagEditCmd {
    pub node_id: DefaultKey,
    pub filename: String,
    pub old_element: TagElement,
    pub new_element: TagElement,
}

impl TagEditCmd {
    pub fn new(node_id: DefaultKey, filename: String, old_element: TagElement, new_element: TagElement) -> Self {
        Self {
            node_id,
            filename,
            old_element,
            new_element,
        }
    }
}

impl AppCmd for TagEditCmd {
    fn undo(&self, app: &mut crate::app::App) {
        let node_id = self.node_id;
        let old_element = self.old_element.clone();
        let old_text = crate::dicom::element_text(&old_element, old_element.header().tag);
        let tag = old_element.header().tag;
        let source = crate::dicom::TagSource {
            tag,
            filename: self.filename.clone(),
        };
        if let Some(dataset) = app.dicom_data.dicom_obj_for_source_mut(&source) {
            dataset.put_element(old_element);
        }
        if let Some(node) = app.tree_widget.nodes.get_mut(node_id) {
            node.text = old_text;
        }
        app.modified_files.insert(self.filename.clone());
        app.handler_text = format!("Undo: reverted {}", source.tag,);
    }

    fn apply(&self, app: &mut crate::app::App) {
        let node_id = self.node_id;
        let new_element = self.new_element.clone();
        let new_text = crate::dicom::element_text(&new_element, new_element.header().tag);
        let tag = new_element.header().tag;
        let source = crate::dicom::TagSource {
            tag,
            filename: self.filename.clone(),
        };

        if let Some(dataset) = app.dicom_data.dicom_obj_for_source_mut(&source) {
            dataset.put_element(new_element);
        }
        if let Some(node) = app.tree_widget.nodes.get_mut(node_id) {
            node.text = new_text;
        }
        app.modified_files.insert(self.filename.clone());
        app.handler_text = format!("Redo: re-applied {}", source.tag,);
    }
}

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
