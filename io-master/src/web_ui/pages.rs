//! The HTML of the pages.
//!
//! Every page is built by a function that returns the whole
//! document as text. Nothing is fetched by the page itself yet;
//! when a page needs live data (the scan, step 5 of
//! `docs/todo.md`), it will ask the JSON API of the server.
//!
//! Text that comes from outside the program (the name of the
//! evaluator, later the names of devices) is always passed
//! through [`escape`], so it can never become HTML or a script.

use crate::installation::installation::{Installation, Role};

/// The pages behind the navigation on the left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Chat,
    Scan,
    Settings,
    Admin,
}

impl Page {
    /// The address of the page.
    pub fn path(self) -> &'static str {
        match self {
            Page::Chat => "/chat",
            Page::Scan => "/scan",
            Page::Settings => "/settings",
            Page::Admin => "/admin",
        }
    }

    /// The title shown in the navigation and above the page.
    pub fn title(self) -> &'static str {
        match self {
            Page::Chat => "Chat",
            Page::Scan => "Scan and permissions",
            Page::Settings => "Settings",
            Page::Admin => "Password reset",
        }
    }

    /// The pages of a role, in the order of the navigation.
    pub fn of_role(role: Role) -> &'static [Page] {
        match role {
            Role::Workstation => &[Page::Chat, Page::Scan, Page::Settings],
            Role::Master => &[Page::Admin],
        }
    }

    /// The page shown after login.
    pub fn home(role: Role) -> Page {
        Page::of_role(role)[0]
    }

    /// The content of a page that does not exist yet.
    ///
    /// TODO: every such page is a placeholder until the step of
    /// `docs/todo.md` it names. Chat, Scan and Settings have pages
    /// of their own (`chat_page.rs`, `scan_page.rs`, `settings.rs`).
    pub fn placeholder(self) -> &'static str {
        match self {
            Page::Chat | Page::Scan | Page::Settings => "",
            Page::Admin => "<p>Password reset requests are approved here (step 11).</p>",
        }
    }
}

/// Makes text safe to put into HTML: the five characters that
/// HTML gives a meaning are replaced by their entities.
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());

    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }

    escaped
}

/// The bars at the top of every page: warnings the user has to
/// see whatever page is open.
///
/// The installation decides which bars are shown; it is not read
/// yet, because both bars are always shown so far: neither the
/// reset service nor the TPM factor can be configured yet.
fn warning_bars(_installation: &Installation) -> String {
    let mut bars = String::new();

    // The more serious warning first, in red: the protection of
    // the data. TODO (step 8 of docs/todo.md): show it only while
    // the TPM factor is off.
    bars.push_str(
        "<div class=\"bar danger\">No TPM is configured: the databases are protected \
         by the passwords alone. Whoever copies the files can try to guess them.</div>",
    );

    // In orange: a forgotten password is a risk the user can avoid.
    // TODO (step 11 of docs/todo.md): show it only while no reset
    // service is configured.
    bars.push_str(
        "<div class=\"bar warning\">No password reset is configured: \
         a forgotten password means lost data.</div>",
    );

    bars
}

/// The frame of an HTML document: head, styles, body.
fn document(title: &str, body: &str) -> String {
    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{} - IO Master</title>\n\
         <link rel=\"stylesheet\" href=\"/static/style.css\">\n\
         </head>\n\
         <body>\n{}\n</body>\n\
         </html>\n",
        escape(title),
        body
    )
}

/// A page of a logged-in evaluator: the warning bars, the
/// navigation on the left and the page on the right, with the given
/// HTML as its content.
pub fn page(installation: &Installation, user: &str, active: Page, content: &str) -> String {
    let links: String = Page::of_role(installation.role)
        .iter()
        .map(|&page| {
            let class = if page == active { " class=\"active\"" } else { "" };

            format!("<a href=\"{}\"{}>{}</a>", page.path(), class, page.title())
        })
        .collect();

    let body = format!(
        "{bars}\n\
         <div class=\"layout\">\n\
         <nav>\n\
         <div class=\"brand\">IO Master</div>\n\
         <div class=\"role\">{role}</div>\n\
         {links}\n\
         <div class=\"user\">{user}\n\
         <form method=\"post\" action=\"/logout\"><button type=\"submit\">Log out</button></form>\n\
         </div>\n\
         </nav>\n\
         <main>\n\
         <h1>{title}</h1>\n\
         {content}\n\
         </main>\n\
         </div>",
        bars = warning_bars(installation),
        role = installation.role.name(),
        links = links,
        user = escape(user),
        title = active.title(),
        content = content,
    );

    document(active.title(), &body)
}

/// The login page, with an explanation when the last attempt was
/// refused.
pub fn login_page(installation: &Installation, problem: Option<&str>) -> String {
    let problem = problem_text(problem);

    let body = format!(
        "{bars}\n\
         <div class=\"login\">\n\
         <h1>IO Master</h1>\n\
         {problem}\n\
         <form method=\"post\" action=\"/login\">\n\
         <label>Name <input name=\"username\" autocomplete=\"username\" required></label>\n\
         <label>Password <input name=\"password\" type=\"password\" \
         autocomplete=\"current-password\"></label>\n\
         <button type=\"submit\">Log in</button>\n\
         </form>\n\
         </div>",
        bars = warning_bars(installation),
        problem = problem,
    );

    document("Log in", &body)
}

/// An explanation of a refused form, or nothing.
pub fn problem_text(problem: Option<&str>) -> String {
    problem
        .map(|text| format!("<p class=\"problem\">{}</p>", escape(text)))
        .unwrap_or_default()
}

/// The page of the first start: the setup code, the first user and
/// the role of the installation.
pub fn setup_page(problem: Option<&str>) -> String {
    let body = format!(
        "<div class=\"login wide\">\n\
         <h1>IO Master: first start</h1>\n\
         <p class=\"note\">Nothing is installed yet. Create the first user and choose \
         the role of this installation.</p>\n\
         {problem}\n\
         <form method=\"post\" action=\"/setup\">\n\
         <label>Setup code <input name=\"code\" autocomplete=\"off\" required>\
         <span class=\"note\">Printed in the terminal where IO Master was started.</span></label>\n\
         <label>Name of the first user <input name=\"username\" autocomplete=\"username\" required></label>\n\
         <label>Password <input name=\"password\" type=\"password\" autocomplete=\"new-password\" \
         minlength=\"12\" required><span class=\"note\">At least 12 characters.</span></label>\n\
         <label>Password again <input name=\"password_repeat\" type=\"password\" \
         autocomplete=\"new-password\" required></label>\n\
         <fieldset>\n\
         <legend>Role of this installation</legend>\n\
         <label class=\"choice\"><input type=\"radio\" name=\"role\" value=\"workstation\" checked> \
         <b>Workstation</b>: the computer of an evaluator. Users with equal rights; \
         any user can add further users.</label>\n\
         <label class=\"choice\"><input type=\"radio\" name=\"role\" value=\"master\"> \
         <b>Master</b>: the password reset service only. You stay its only user.</label>\n\
         </fieldset>\n\
         <p class=\"note\">A master must run on its own computer, never on the computer \
         of an evaluator. A workstation works without a master; the connection to \
         a master (password reset) can be set or changed later in Settings.</p>\n\
         <label class=\"choice\"><input type=\"checkbox\" name=\"confirm\" required> \
         I understand that the role cannot be changed later.</label>\n\
         <button type=\"submit\">Install</button>\n\
         </form>\n\
         </div>",
        problem = problem_text(problem),
    );

    document("First start", &body)
}

/// The page for an address that does not exist, or that belongs
/// to the other role.
pub fn not_found_page() -> String {
    document(
        "Not found",
        "<div class=\"login\"><h1>Not found</h1><p><a href=\"/\">Back to IO Master</a></p></div>",
    )
}

// Unit tests, in their own file to keep this one readable. They
// are part of this module, so they can use its private items.
#[cfg(test)]
#[path = "pages_tests.rs"]
mod tests;
