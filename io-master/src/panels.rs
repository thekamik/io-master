//! `panels/`: the panels of the window (user, 2026-10-28), each in two layers
//! with the same folders: views/ (only the commands the page calls, and
//! their error) and logic/ (the work behind them, not called by the page; it
//! gets the open database as a parameter and never reaches the session).

// What every command of the window shares: the data folder, and the user
// logged in with their open database (the only global state).
pub mod logic;
pub mod session;
pub mod views;
