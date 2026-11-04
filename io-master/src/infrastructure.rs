//! `infrastructure/`: what belongs to no object and no panel, and knows no
//! object (user, 2026-11-01): the database files and their encryption, random
//! bytes and the protection of the process, PDFs read with Poppler, the clock,
//! the data folder.

// The time of the computer, as every time written into the databases
// is read.
pub mod clock;
// Where the data of the program is: $XDG_DATA_HOME/io-master.
pub mod data_folder;
pub mod pdf;
pub mod security;
pub mod storage;
