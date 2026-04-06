use dicom_core::DataElement;
use dicom_object::InMemDicomObject;
use slotmap::DefaultKey;

use crate::dicom::TagElement;

pub trait AppCmd: std::fmt::Debug {
    fn execute(&self, app: &mut crate::app::App);
    fn undo(&self, app: &mut crate::app::App);
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MacroCmd<Cmd: AppCmd> {
    cmds: Vec<Cmd>,
}

impl<Cmd: AppCmd> MacroCmd<Cmd> {
    pub fn _new(cmds: Vec<Cmd>) -> Self {
        Self { cmds }
    }
}

impl<Cmd: AppCmd> AppCmd for MacroCmd<Cmd> {
    fn execute(&self, app: &mut crate::app::App) {
        for cmd in &self.cmds {
            cmd.execute(app);
        }
    }

    fn undo(&self, app: &mut crate::app::App) {
        for cmd in self.cmds.iter().rev() {
            cmd.undo(app);
        }
    }
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

    fn apply(&self, app: &mut crate::app::App, element: DataElement<InMemDicomObject>, helper_text: &str) {
        let node_id = self.node_id;
        let text = crate::dicom::element_text(&element, element.header().tag);
        let tag = element.header().tag;
        let source = crate::dicom::TagSource {
            tag,
            filename: self.filename.clone(),
        };
        if let Some(dataset) = app.dicom_data.dicom_obj_for_source_mut(&source) {
            dataset.put_element(element);
        }
        if let Some(node) = app.tree_widget.nodes.get_mut(node_id) {
            node.text = text;
        }
        app.modified_files.insert(self.filename.clone());
        app.handler_text = format!("{helper_text} {}", source.tag,);
    }
}

impl AppCmd for TagEditCmd {
    fn undo(&self, app: &mut crate::app::App) {
        self.apply(app, self.old_element.clone(), "Undo: reverted");
    }

    fn execute(&self, app: &mut crate::app::App) {
        self.apply(app, self.new_element.clone(), "Update: applied");
    }
}
