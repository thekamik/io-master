//! A column of the table of ports, as both elements of the view show
//! it: the table above its values, the settings as a choice.

use serde::Serialize;

use crate::hardware_tests::devices::port_property::PortProperty;

/// A column that can be shown.
#[derive(Debug, Serialize)]
pub struct ColumnView {
    /// The key stored in the settings: `"path"`.
    pub key: &'static str,
    /// The title above the column: `"Path"`.
    pub title: &'static str,
}

impl ColumnView {
    pub fn new(property: PortProperty) -> ColumnView {
        ColumnView { key: property.key(), title: property.title() }
    }

    /// The views of `properties`, in their order.
    pub fn all(properties: &[PortProperty]) -> Vec<ColumnView> {
        let mut columns = Vec::with_capacity(properties.len());

        for property in properties {
            columns.push(ColumnView::new(*property));
        }

        columns
    }
}
