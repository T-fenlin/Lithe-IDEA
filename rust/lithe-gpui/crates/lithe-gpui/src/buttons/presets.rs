use gpui::ElementId;

use super::Button;
use crate::primitives::IconName;

/// Closes a panel, dialog or tab.
pub fn close_button(id: impl Into<ElementId>) -> Button {
    Button::new(id).icon(IconName::X).tooltip("Close")
}

/// Returns to the previous view.
pub fn back_button(id: impl Into<ElementId>) -> Button {
    Button::new(id).icon(IconName::ArrowLeft).tooltip("Back")
}

/// Opens more actions.
pub fn more_button(id: impl Into<ElementId>) -> Button {
    Button::new(id).icon(IconName::Ellipsis).tooltip("More")
}
