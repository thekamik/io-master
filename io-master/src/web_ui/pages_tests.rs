//! Unit tests of `pages.rs`.

use super::*;

fn workstation() -> Config {
    Config {
        role: Role::Workstation,
        single_host: false,
    }
}

#[test]
fn text_is_escaped() {
    assert_eq!(
        escape("<script>alert('x')</script> & \"y\""),
        "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt; &amp; &quot;y&quot;"
    );
}

#[test]
fn the_navigation_holds_the_pages_of_the_role() {
    let html = page(&workstation(), "anna", Page::Scan);

    for page in [Page::Chat, Page::Scan, Page::Settings] {
        assert!(html.contains(page.path()), "{page:?} is missing");
    }

    assert!(!html.contains(Page::Admin.path()));
    assert!(html.contains("<a href=\"/scan\" class=\"active\">"));
}

#[test]
fn the_master_sees_only_its_panel() {
    let master = Config {
        role: Role::Master,
        ..workstation()
    };

    let html = page(&master, "admin", Page::Admin);

    assert!(html.contains("/admin"));
    assert!(!html.contains("/chat"));
}

#[test]
fn the_name_of_the_evaluator_cannot_inject_html() {
    let html = page(&workstation(), "<img src=x>", Page::Chat);

    assert!(html.contains("&lt;img src=x&gt;"));
    assert!(!html.contains("<img src=x>"));
}

#[test]
fn the_missing_reset_is_always_shown() {
    assert!(page(&workstation(), "anna", Page::Chat).contains("No password reset is configured"));
    assert!(login_page(&workstation(), None).contains("No password reset is configured"));
}

#[test]
fn single_host_mode_is_shown_only_when_on() {
    let single = Config {
        single_host: true,
        ..workstation()
    };

    assert!(page(&single, "anna", Page::Chat).contains("Single-host mode"));
    assert!(!page(&workstation(), "anna", Page::Chat).contains("Single-host mode"));
}
