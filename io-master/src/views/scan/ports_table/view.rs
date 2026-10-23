//! The data of the table of ports, as the page gets it: a ready table
//! of texts (the titles of the columns, the values of each port), so
//! that the page knows nothing of the structure of a port.

use serde::Serialize;

use crate::app::error::UiError;
use crate::hardware_tests::devices::port_property::PortProperty;
use crate::hardware_tests::devices::scan_settings::MAX_COLUMNS;
use crate::hardware_tests::devices::serial_ports::{ScanError, SerialPortInfo};

use super::super::column_view::ColumnView;

/// The most ports the table shows. A computer has a few serial ports,
/// a laboratory rack some dozens; more is a fault of the system.
pub const MAX_PORTS: usize = 1024;

/// The table of ports.
#[derive(Debug, Serialize)]
pub struct PortsTableView {
    /// The columns, in the order of the settings.
    pub columns: Vec<ColumnView>,
    /// One row per port, one text per column. Values of several lines
    /// (the stable names) keep their line breaks. Empty when the scan
    /// failed.
    pub rows: Vec<Vec<String>>,
    /// Why the ports could not be scanned, as a sentence for the
    /// evaluator; `None` when the scan worked.
    pub problem: Option<String>,
}

impl PortsTableView {
    /// The table of a scan with `columns`: its ports, or why it failed.
    pub fn new(
        columns: &[PortProperty],
        outcome: Result<Vec<SerialPortInfo>, ScanError>,
    ) -> Result<PortsTableView, UiError> {
        ensure!(!columns.is_empty(), UiError::Failed("the table of ports has no columns".to_owned()));
        ensure!(columns.len() <= MAX_COLUMNS, UiError::Failed("the table of ports has too many columns".to_owned()));

        let ports = match outcome {
            Ok(ports) => ports,
            Err(error) => {
                let problem = Some(format!("The ports cannot be scanned: {error}."));

                return Ok(PortsTableView { columns: ColumnView::all(columns), rows: Vec::new(), problem });
            }
        };

        ensure!(ports.len() <= MAX_PORTS, UiError::Failed(format!("the scan found {} ports", ports.len())));

        Ok(PortsTableView { columns: ColumnView::all(columns), rows: rows(&ports, columns), problem: None })
    }
}

/// The values of `columns` for every port.
fn rows(ports: &[SerialPortInfo], columns: &[PortProperty]) -> Vec<Vec<String>> {
    let mut rows = Vec::with_capacity(ports.len());

    for port in ports {
        let mut row = Vec::with_capacity(columns.len());

        for column in columns {
            row.push(column.value(port));
        }

        rows.push(row);
    }

    rows
}
