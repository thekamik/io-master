//! The settings of the scanner: which kinds of port a scan looks for,
//! and which properties the table of ports shows.
//!
//! Plain data and its rules, without any I/O. They are stored by
//! `db/scan_settings/` as two texts, and used by the elements of the
//! view Scan (`views/devices/`):
//!
//! ```text
//! a form of the page   ScanSettings::from_form()     strict: what the page never sends is refused
//! the stored texts     ScanSettings::from_stored()   forgiving: what cannot be used is passed over
//! back to the texts    stored_port_types(), stored_columns()
//! ```
//!
//! The stored texts:
//!
//! ```text
//! port types   "all", or base names: "ttyACM,ttyUSB"
//! columns      keys of PortProperty: "path,type,device"
//! ```
//!
//! Reading the stored texts is forgiving on purpose: an unknown column
//! (removed by a later version) is passed over, and texts that cannot be
//! used at all give the defaults, so a stored choice never stops the
//! page from working.

use std::fmt;

use super::port_property::PortProperty;
use super::serial_ports::{PortFilter, PortType};

/// The most kinds of port that can be chosen (31 are known; the rest
/// leaves room for kinds this version has no name for).
pub const MAX_PORT_TYPES: usize = 64;

/// The longest base name of a kind of port (the longest known one,
/// `ttyRPMSG`, has 8 characters).
pub const MAX_BASE_NAME: usize = 32;

/// The most columns: every property of a port once.
pub const MAX_COLUMNS: usize = PortProperty::ALL.len();

/// The stored text for "every kind of port".
const ALL: &str = "all";

/// The settings of the scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanSettings {
    /// Which ports a scan reports.
    pub filter: PortFilter,
    /// Which properties the table shows, in this order. Never empty.
    pub columns: Vec<PortProperty>,
}

/// Why a choice made in a form is refused. The page offers only what
/// is valid, so each of these is a mistake of the page, never of the
/// evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanSettingsError {
    /// More kinds of port than [`MAX_PORT_TYPES`]; holds the number.
    TooManyTypes(usize),
    /// A name that cannot be the base name of a port.
    BadBaseName(String),
    /// More columns than there are properties; holds the number.
    TooManyColumns(usize),
    /// A key that is not a property of a port.
    UnknownColumn(String),
}

impl fmt::Display for ScanSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanSettingsError::TooManyTypes(count) => {
                write!(formatter, "{count} kinds of port were chosen; at most {MAX_PORT_TYPES} can be")
            }
            ScanSettingsError::BadBaseName(name) => write!(formatter, "\"{name}\" is not the name of a kind of port"),
            ScanSettingsError::TooManyColumns(count) => {
                write!(formatter, "{count} columns were chosen; the table has only {MAX_COLUMNS}")
            }
            ScanSettingsError::UnknownColumn(key) => write!(formatter, "\"{key}\" is not a column of the table"),
        }
    }
}

impl std::error::Error for ScanSettingsError {}

impl Default for ScanSettings {
    /// Every port, with the default columns.
    fn default() -> ScanSettings {
        ScanSettings { filter: PortFilter::All, columns: PortProperty::DEFAULT.to_vec() }
    }
}

impl ScanSettings {
    /// The settings chosen in the form of the page: all kinds of port,
    /// or the kinds named in `types`; the columns named by their keys
    /// in `columns`. Nothing chosen means every kind of port and the
    /// default columns: an empty table is never what was meant.
    pub fn from_form(all_types: bool, types: &[String], columns: &[String]) -> Result<ScanSettings, ScanSettingsError> {
        ensure!(types.len() <= MAX_PORT_TYPES, ScanSettingsError::TooManyTypes(types.len()));
        ensure!(columns.len() <= MAX_COLUMNS, ScanSettingsError::TooManyColumns(columns.len()));

        let types = form_types(types)?;
        let columns = form_columns(columns)?;

        Ok(ScanSettings::chosen(all_types, types, &columns))
    }

    /// The settings from the stored texts (`None`: never stored).
    pub fn from_stored(port_types: Option<&str>, columns: Option<&str>) -> ScanSettings {
        let defaults = ScanSettings::default();

        let filter = match port_types {
            None => defaults.filter,
            Some(text) => stored_filter(text),
        };

        let columns = match columns {
            None => defaults.columns,
            Some(text) => stored_columns(text),
        };

        ScanSettings { filter, columns }
    }

    /// The kinds of port as they are stored.
    pub fn stored_port_types(&self) -> String {
        let PortFilter::Types(types) = &self.filter else {
            return ALL.to_owned();
        };

        let mut names = Vec::with_capacity(types.len());

        for port_type in types {
            names.push(port_type.base_name());
        }

        names.join(",")
    }

    /// The columns as they are stored.
    pub fn stored_columns(&self) -> String {
        let mut keys = Vec::with_capacity(self.columns.len());

        for column in &self.columns {
            keys.push(column.key());
        }

        keys.join(",")
    }

    /// The settings of what was chosen; nothing chosen means every kind
    /// of port and the default columns.
    fn chosen(all_types: bool, types: Vec<PortType>, columns: &[PortProperty]) -> ScanSettings {
        let filter = if all_types || types.is_empty() { PortFilter::All } else { PortFilter::Types(types) };
        let columns = if columns.is_empty() { PortProperty::DEFAULT.to_vec() } else { in_table_order(columns) };

        ScanSettings { filter, columns }
    }
}

/// The kinds of port named in a form; a name that cannot be one is
/// refused.
fn form_types(names: &[String]) -> Result<Vec<PortType>, ScanSettingsError> {
    let mut types = Vec::with_capacity(names.len());

    for name in names {
        ensure!(is_base_name(name), ScanSettingsError::BadBaseName(name.clone()));
        types.push(PortType::from_base_name(name));
    }

    Ok(types)
}

/// The columns named by their keys in a form; an unknown key is refused.
fn form_columns(keys: &[String]) -> Result<Vec<PortProperty>, ScanSettingsError> {
    let mut columns = Vec::with_capacity(keys.len());

    for key in keys {
        let Some(column) = PortProperty::from_key(key) else {
            return Err(ScanSettingsError::UnknownColumn(key.clone()));
        };

        columns.push(column);
    }

    Ok(columns)
}

/// Whether `name` can be the base name of a port: 1 to
/// [`MAX_BASE_NAME`] ASCII letters and digits (`ttyACM`, `rfcomm`).
fn is_base_name(name: &str) -> bool {
    if name.is_empty() || name.len() > MAX_BASE_NAME {
        return false;
    }

    for byte in name.bytes() {
        if !byte.is_ascii_alphanumeric() {
            return false;
        }
    }

    true
}

/// The chosen columns in the order of the table, whatever order they
/// came in.
fn in_table_order(chosen: &[PortProperty]) -> Vec<PortProperty> {
    let mut columns = Vec::with_capacity(chosen.len());

    for property in PortProperty::ALL {
        if chosen.contains(&property) {
            columns.push(property);
        }
    }

    columns
}

/// The filter of the stored text; names that cannot be used are passed
/// over, and nothing usable means every kind of port.
fn stored_filter(text: &str) -> PortFilter {
    if text == ALL {
        return PortFilter::All;
    }

    let mut types = Vec::new();

    for name in text.split(',').take(MAX_PORT_TYPES) {
        if is_base_name(name) {
            types.push(PortType::from_base_name(name));
        }
    }

    if types.is_empty() { PortFilter::All } else { PortFilter::Types(types) }
}

/// The columns of the stored text, in their stored order; unknown keys
/// are passed over, and nothing usable means the default columns.
fn stored_columns(text: &str) -> Vec<PortProperty> {
    let mut columns = Vec::new();

    for key in text.split(',').take(MAX_COLUMNS) {
        if let Some(column) = PortProperty::from_key(key) {
            columns.push(column);
        }
    }

    if columns.is_empty() { PortProperty::DEFAULT.to_vec() } else { columns }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `scan_settings.rs`.

    use super::*;

    /// The texts of a form.
    fn texts(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_form_gives_the_kinds_and_the_columns_in_the_order_of_the_table() {
        let settings =
            ScanSettings::from_form(false, &texts(&["ttyACM", "ttyUSB"]), &texts(&["driver", "path"])).unwrap();

        assert_eq!(settings.filter, PortFilter::Types(vec![PortType::UsbAcm, PortType::UsbSerial]));
        assert_eq!(settings.columns, [PortProperty::Path, PortProperty::Driver]);
    }

    #[test]
    fn an_empty_form_means_everything_with_the_default_columns() {
        assert_eq!(ScanSettings::from_form(false, &[], &[]).unwrap(), ScanSettings::default());
        assert_eq!(ScanSettings::from_form(true, &texts(&["ttyACM"]), &[]).unwrap(), ScanSettings::default());
    }

    #[test]
    fn a_kind_without_a_name_in_this_version_is_kept() {
        let settings = ScanSettings::from_form(false, &texts(&["ttyRP"]), &[]).unwrap();

        assert_eq!(settings.filter, PortFilter::Types(vec![PortType::Other("ttyRP".to_owned())]));
    }

    #[test]
    fn a_form_with_what_the_page_never_sends_is_refused() {
        let refused = [
            (texts(&[""]), Vec::new(), ScanSettingsError::BadBaseName(String::new())),
            (texts(&["tty/ACM"]), Vec::new(), ScanSettingsError::BadBaseName("tty/ACM".to_owned())),
            (Vec::new(), texts(&["path", "not_a_column"]), ScanSettingsError::UnknownColumn("not_a_column".to_owned())),
            (
                vec!["ttyS".to_owned(); MAX_PORT_TYPES + 1],
                Vec::new(),
                ScanSettingsError::TooManyTypes(MAX_PORT_TYPES + 1),
            ),
            (Vec::new(), vec!["path".to_owned(); MAX_COLUMNS + 1], ScanSettingsError::TooManyColumns(MAX_COLUMNS + 1)),
        ];

        for (types, columns, error) in refused {
            assert_eq!(ScanSettings::from_form(false, &types, &columns), Err(error));
        }
    }

    #[test]
    fn the_texts_are_read_back_as_they_were_stored() {
        let settings =
            ScanSettings::from_form(false, &texts(&["ttyACM", "ttyRP"]), &texts(&["path", "driver"])).unwrap();

        assert_eq!(settings.stored_port_types(), "ttyACM,ttyRP");
        assert_eq!(settings.stored_columns(), "path,driver");

        let read = ScanSettings::from_stored(Some(&settings.stored_port_types()), Some(&settings.stored_columns()));
        assert_eq!(read, settings);

        assert_eq!(ScanSettings::default().stored_port_types(), "all");
        assert_eq!(ScanSettings::from_stored(Some("all"), None), ScanSettings::default());
    }

    #[test]
    fn nothing_stored_gives_the_defaults() {
        assert_eq!(ScanSettings::from_stored(None, None), ScanSettings::default());
    }

    #[test]
    fn stored_texts_that_cannot_be_used_are_passed_over() {
        let settings = ScanSettings::from_stored(Some("ttyACM,,tty/x"), Some("removed_column,path,also_unknown"));
        assert_eq!(settings.filter, PortFilter::Types(vec![PortType::UsbAcm]));
        assert_eq!(settings.columns, [PortProperty::Path]);

        assert_eq!(ScanSettings::from_stored(Some(","), Some("nothing,known")), ScanSettings::default());
    }
}
