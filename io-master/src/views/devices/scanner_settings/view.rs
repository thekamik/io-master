//! The data of the settings of the scanner, as the page gets it: the
//! settings, their summary in one line, and what can be chosen.

use serde::Serialize;

use crate::hardware_tests::devices::port_property::PortProperty;
use crate::hardware_tests::devices::scan_settings::ScanSettings;
use crate::hardware_tests::devices::serial_ports::{PortFilter, PortType};

use super::super::column_view::ColumnView;

/// The settings of the scanner.
#[derive(Debug, Serialize)]
pub struct ScannerSettingsView {
    /// Every kind of port is scanned.
    pub all_types: bool,
    /// The chosen kinds (base names) when not all.
    pub types: Vec<String>,
    /// The keys of the chosen columns, in the order of the table.
    pub columns: Vec<&'static str>,
    /// The settings in one line: "ttyACM · 5 columns".
    pub summary: String,
    /// What can be chosen.
    pub choices: ScannerChoicesView,
}

/// What can be chosen in the form.
#[derive(Debug, Serialize)]
pub struct ScannerChoicesView {
    /// The base names of every known kind of port, and of the unknown
    /// ones that were chosen.
    pub types: Vec<String>,
    /// Every column.
    pub columns: Vec<ColumnView>,
}

impl ScannerSettingsView {
    pub fn new(settings: &ScanSettings) -> ScannerSettingsView {
        let all_types = settings.filter == PortFilter::All;
        let types = chosen_types(&settings.filter);

        let mut columns = Vec::with_capacity(settings.columns.len());

        for column in &settings.columns {
            columns.push(column.key());
        }

        ScannerSettingsView {
            all_types,
            summary: summary(&types, columns.len()),
            types,
            columns,
            choices: ScannerChoicesView {
                types: type_choices(&settings.filter),
                columns: ColumnView::all(&PortProperty::ALL),
            },
        }
    }
}

/// The base names of the chosen kinds; none when every kind is scanned.
fn chosen_types(filter: &PortFilter) -> Vec<String> {
    let PortFilter::Types(types) = filter else {
        return Vec::new();
    };

    let mut names = Vec::with_capacity(types.len());

    for port_type in types {
        names.push(port_type.base_name().to_owned());
    }

    names
}

/// The settings in one line: "ttyACM, ttyUSB · 2 columns", or
/// "all kinds of port · 5 columns" when `types` is empty.
fn summary(types: &[String], columns: usize) -> String {
    let kinds = if types.is_empty() { "all kinds of port".to_owned() } else { types.join(", ") };
    let plural = if columns == 1 { "" } else { "s" };

    format!("{kinds} · {columns} column{plural}")
}

/// Every known kind of port, then the unknown kinds that were chosen,
/// so that a choice is never lost from the form.
fn type_choices(filter: &PortFilter) -> Vec<String> {
    let mut names = Vec::new();

    for port_type in PortType::known() {
        names.push(port_type.base_name().to_owned());
    }

    for name in chosen_types(filter) {
        if !names.contains(&name) {
            names.push(name);
        }
    }

    names
}
