//! The HTML of the pages.
//!
//! Every page is built by a function that returns the whole
//! document as text. Nothing is fetched by the page itself yet;
//! when a page needs live data (the scan, step 2 of
//! `docs/todo.md`), it will ask the JSON API of the server.
//!
//! Text that comes from outside the program (the name of the
//! evaluator, later the names of devices) is always passed
//! through [`escape`], so it can never become HTML or a script.

use crate::config::config::{Config, Role};

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

    /// The content of the page.
    ///
    /// TODO: every page is a placeholder until the step of
    /// `docs/todo.md` it names.
    fn content(self) -> &'static str {
        match self {
            Page::Chat => "<p>The AI assistant comes here (step 6).</p>",
            Page::Scan => {
                "<p>The serial ports and the access rights to them come here (steps 2 and 3).</p>"
            }
            Page::Settings => {
                "<p>The choice of the columns of the port table comes here (step 2).</p>"
            }
            Page::Admin => "<p>Password reset requests are approved here (step 7).</p>",
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
fn warning_bars(config: &Config) -> String {
    let mut bars = String::new();

    // TODO (step 7): show it only while no reset service is
    // configured; until then none can be.
    bars.push_str(
        "<div class=\"bar danger\">No password reset is configured: \
         a forgotten password means lost data.</div>",
    );

    if config.single_host {
        bars.push_str(
            "<div class=\"bar warning\">Single-host mode: master and workstation \
             run on one computer. Not recommended: whoever takes over this computer \
             gets every secret.</div>",
        );
    }

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
/// navigation on the left and the page on the right.
pub fn page(config: &Config, user: &str, active: Page) -> String {
    let links: String = Page::of_role(config.role)
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
        bars = warning_bars(config),
        role = config.role.name(),
        links = links,
        user = escape(user),
        title = active.title(),
        content = active.content(),
    );

    document(active.title(), &body)
}

/// The login page, with an explanation when the last attempt was
/// refused.
pub fn login_page(config: &Config, problem: Option<&str>) -> String {
    let problem = problem
        .map(|text| format!("<p class=\"problem\">{}</p>", escape(text)))
        .unwrap_or_default();

    let body = format!(
        "{bars}\n\
         <div class=\"login\">\n\
         <h1>IO Master</h1>\n\
         <p class=\"note\">Mock-up login: any name is accepted and the password \
         is not checked yet.</p>\n\
         {problem}\n\
         <form method=\"post\" action=\"/login\">\n\
         <label>Name <input name=\"username\" autocomplete=\"username\" required></label>\n\
         <label>Password <input name=\"password\" type=\"password\" \
         autocomplete=\"current-password\"></label>\n\
         <button type=\"submit\">Log in</button>\n\
         </form>\n\
         </div>",
        bars = warning_bars(config),
        problem = problem,
    );

    document("Log in", &body)
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
