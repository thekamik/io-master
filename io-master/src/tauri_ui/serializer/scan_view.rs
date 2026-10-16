//! The scan page as the page shows it: the settings of the scanner,
//! what can be chosen, and the table of ports.
//!
//! The ports are sent as a ready table (column titles and rows of
//! text), made with `PortProperty::value`, so that the page shows
//! exactly what the web interface showed and knows nothing of the
//! structure of a port.

use serde::Serialize;

use crate::devices::devices::{PortFilter, PortType, SerialPortInfo};
use crate::devices::port_property::PortProperty;
use crate::devices::scan_settings::ScanSettings;

/// A column that can be shown.
#[derive(Debug, Serialize)]
pub struct ColumnView {
    /// The key stored in the settings: `"path"`.
    pub key: &'static str,
    /// The title above the column: `"Path"`.
    pub title: &'static str,
}

impl From<PortProperty> for ColumnView {
    fn from(property: PortProperty) -> ColumnView {
        ColumnView {
            key: property.key(),
            title: property.title(),
        }
    }
}

/// The settings of the scanner.
#[derive(Debug, Serialize)]
pub struct ScanSettingsView {
    /// Every kind of port is scanned.
    pub all_types: bool,
    /// The chosen kinds (base names) when not all.
    pub types: Vec<String>,
    /// The keys of the chosen columns, in the order of the table.
    pub columns: Vec<&'static str>,
    /// The settings in one line: "ttyACM · 5 columns".
    pub summary: String,
}

/// What can be chosen.
#[derive(Debug, Serialize)]
pub struct ScanChoicesView {
    /// The base names of every known kind of port, and of the
    /// unknown ones that were chosen.
    pub types: Vec<String>,
    pub columns: Vec<ColumnView>,
}

/// The table of ports with the chosen columns.
#[derive(Debug, Serialize)]
pub struct PortTableView {
    pub columns: Vec<ColumnView>,
    /// One row per port; one text per column. Values of several
    /// lines (the stable names) keep their line breaks.
    pub rows: Vec<Vec<String>>,
}

/// The whole scan page.
#[derive(Debug, Serialize)]
pub struct ScanView {
    pub settings: ScanSettingsView,
    pub choices: ScanChoicesView,
    /// The ports; `None` when the scan failed (`problem` says why).
    pub table: Option<PortTableView>,
    pub problem: Option<String>,
}

impl ScanView {
    /// The view of a scan with the given settings and outcome.
    pub fn new(settings: &ScanSettings, outcome: Result<Vec<SerialPortInfo>, String>) -> ScanView {
        let (table, problem) = match outcome {
            Ok(ports) => (Some(table(&ports, &settings.columns)), None),
            Err(problem) => (None, Some(problem)),
        };

        ScanView {
            settings: settings_view(settings),
            choices: choices(settings),
            table,
            problem,
        }
    }
}

fn settings_view(settings: &ScanSettings) -> ScanSettingsView {
    let (all_types, types) = match &settings.filter {
        PortFilter::All => (true, Vec::new()),
        PortFilter::Types(types) => (false, types.iter().map(|port_type| port_type.base_name().to_owned()).collect()),
    };

    let count = settings.columns.len();
    let kinds = if all_types { "all kinds of port".to_owned() } else { types.join(", ") };

    ScanSettingsView {
        all_types,
        types,
        columns: settings.columns.iter().map(|column| column.key()).collect(),
        summary: format!("{kinds} · {count} column{}", if count == 1 { "" } else { "s" }),
    }
}

fn choices(settings: &ScanSettings) -> ScanChoicesView {
    let mut types: Vec<String> = PortType::known().map(|port_type| port_type.base_name().to_owned()).collect();

    // Unknown kinds that were chosen stay in the list.
    if let PortFilter::Types(chosen) = &settings.filter {
        for port_type in chosen {
            if !types.iter().any(|name| name == port_type.base_name()) {
                types.push(port_type.base_name().to_owned());
            }
        }
    }

    ScanChoicesView {
        types,
        columns: PortProperty::ALL.into_iter().map(ColumnView::from).collect(),
    }
}

fn table(ports: &[SerialPortInfo], columns: &[PortProperty]) -> PortTableView {
    PortTableView {
        columns: columns.iter().copied().map(ColumnView::from).collect(),
        rows: ports
            .iter()
            .map(|port| columns.iter().map(|column| column.value(port)).collect())
            .collect(),
    }
}
