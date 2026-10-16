//! Unit tests of `web_ui.rs`, `setup.rs` and `settings.rs`: the
//! routes, as a browser sees them, on a real installation in
//! a temporary directory.

use super::*;

use axum::http::Method;
use axum::http::header::LOCATION;

use crate::installation::installation::Role;
use crate::installation::lifecycle;
use crate::web_ui::test_support::{
    CODE, PASSWORD, first_start, installed, log_in, request, send, text, token_of,
};

/// The fields of a complete setup form, with the given role.
fn setup_form(role: &str) -> String {
    let password = PASSWORD.replace(' ', "+");

    format!("code={CODE}&username=anna&password={password}&password_repeat={password}&role={role}&confirm=on")
}

/* ---------------------------------------------------------------
 * The first start
 * ------------------------------------------------------------- */

#[tokio::test]
async fn before_the_installation_everything_leads_to_the_setup() {
    let server = first_start();

    for path in ["/", "/chat", "/login", "/admin", "/no-such-page"] {
        let response = send(&server.state, request(Method::GET, path, None, "")).await;

        assert_eq!(response.headers()[LOCATION], "/setup", "{path}");
    }

    let response = send(&server.state, request(Method::GET, "/setup", None, "")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(text(response).await.contains("Setup code"));
}

#[tokio::test]
async fn a_wrong_setup_code_is_refused() {
    let server = first_start();

    let form = setup_form("workstation").replace(CODE, "WRONGCODE2");
    let response = send(&server.state, request(Method::POST, "/setup", None, &form)).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(server.state.running().is_none());
}

#[tokio::test]
async fn an_incomplete_setup_is_refused() {
    let server = first_start();

    for form in [
        setup_form("workstation").replace("password_repeat=correct", "password_repeat=other"),
        setup_form("workstation").replace("&confirm=on", ""),
        setup_form("admin"),
        setup_form("workstation").replace("username=anna", "username=+"),
        setup_form("workstation").replace("correct+horse+battery", "short"),
    ] {
        let response = send(&server.state, request(Method::POST, "/setup", None, &form)).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{form}");
    }

    // Nothing was made, also not in the TPM.
    assert!(server.state.running().is_none());
    assert!(matches!(
        lifecycle::open(server.directory.path()).unwrap(),
        lifecycle::Opened::FirstRun
    ));
}

#[tokio::test]
async fn the_setup_installs_and_logs_the_first_user_in() {
    let server = first_start();

    let response = send(&server.state, request(Method::POST, "/setup", None, &setup_form("workstation"))).await;

    assert_eq!(response.headers()[LOCATION], "/");

    // The first user is in, and lands on the first page.
    let token = token_of(&response);
    let response = send(&server.state, request(Method::GET, "/", Some(&token), "")).await;
    assert_eq!(response.headers()[LOCATION], "/chat");

    let response = send(&server.state, request(Method::GET, "/chat", Some(&token), "")).await;
    assert_eq!(response.status(), StatusCode::OK);

    // The installation is in the database and survives a restart.
    let lifecycle::Opened::Installed(running) =
        lifecycle::open(server.directory.path()).unwrap()
    else {
        panic!("nothing was installed");
    };
    assert_eq!(running.installation.role, Role::Workstation);
    assert_eq!(running.system_db.user_names().unwrap(), ["anna"]);
}

#[tokio::test]
async fn after_the_setup_the_choice_is_gone() {
    let server = first_start();

    send(&server.state, request(Method::POST, "/setup", None, &setup_form("master"))).await;

    // The setup page leads home, and a second setup changes nothing.
    let response = send(&server.state, request(Method::GET, "/setup", None, "")).await;
    assert_eq!(response.headers()[LOCATION], "/");

    let response = send(&server.state, request(Method::POST, "/setup", None, &setup_form("workstation"))).await;
    assert_eq!(response.headers()[LOCATION], "/");

    assert_eq!(server.state.running().unwrap().installation.role, Role::Master);
}

/* ---------------------------------------------------------------
 * Login
 * ------------------------------------------------------------- */

#[tokio::test]
async fn pages_need_a_login() {
    let server = installed(Role::Workstation, &["anna"]);

    for path in ["/", "/chat", "/scan", "/settings"] {
        let response = send(&server.state, request(Method::GET, path, None, "")).await;

        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
        assert_eq!(response.headers()[LOCATION], "/login", "{path}");
    }
}

#[tokio::test]
async fn a_wrong_password_and_an_unknown_user_are_refused_alike() {
    let server = installed(Role::Workstation, &["anna"]);

    for body in ["username=anna&password=wrong+horse+battery", "username=mallory&password=correct+horse+battery"] {
        let response = send(&server.state, request(Method::POST, "/login", None, body)).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{body}");
        assert!(text(response).await.contains("Wrong name or password."));
    }
}

#[tokio::test]
async fn a_logged_in_user_lands_on_the_first_page() {
    let server = installed(Role::Workstation, &["anna"]);
    let token = log_in(&server.state, "anna").await;

    let response = send(&server.state, request(Method::GET, "/", Some(&token), "")).await;
    assert_eq!(response.headers()[LOCATION], "/chat");

    let response = send(&server.state, request(Method::GET, "/scan", Some(&token), "")).await;
    let html = text(response).await;

    assert!(html.contains("Scan and permissions"));
    assert!(html.contains("anna"));
    assert!(html.contains("No password reset is configured"));
}

#[tokio::test]
async fn the_home_page_depends_on_the_role() {
    for (role, home) in [(Role::Workstation, "/chat"), (Role::Master, "/admin")] {
        let server = installed(role, &["anna"]);
        let token = log_in(&server.state, "anna").await;

        let response = send(&server.state, request(Method::GET, "/", Some(&token), "")).await;

        assert_eq!(response.headers()[LOCATION], home, "{role:?}");
    }
}

#[tokio::test]
async fn each_role_refuses_the_pages_of_the_other() {
    let workstation = installed(Role::Workstation, &["anna"]);
    let token = log_in(&workstation.state, "anna").await;
    let response = send(&workstation.state, request(Method::GET, "/admin", Some(&token), "")).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let master = installed(Role::Master, &["admin"]);
    let token = log_in(&master.state, "admin").await;

    for path in ["/chat", "/scan", "/settings"] {
        let response = send(&master.state, request(Method::GET, path, Some(&token), "")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn logout_ends_the_session() {
    let server = installed(Role::Workstation, &["anna"]);
    let token = log_in(&server.state, "anna").await;

    let response = send(&server.state, request(Method::POST, "/logout", Some(&token), "")).await;

    assert_eq!(response.headers()[LOCATION], "/login");
    assert!(response.headers()[SET_COOKIE].to_str().unwrap().contains("Max-Age=0"));

    // The old token no longer opens a page.
    let response = send(&server.state, request(Method::GET, "/chat", Some(&token), "")).await;
    assert_eq!(response.headers()[LOCATION], "/login");
}

/* ---------------------------------------------------------------
 * Scan
 * ------------------------------------------------------------- */

#[tokio::test]
async fn the_scan_page_shows_the_settings_and_the_ports() {
    let server = installed(Role::Workstation, &["anna"]);
    let token = log_in(&server.state, "anna").await;

    let response = send(&server.state, request(Method::GET, "/scan", Some(&token), "")).await;
    assert_eq!(response.status(), StatusCode::OK);

    let html = text(response).await;

    // The defaults of a new account; the result depends on the
    // hardware of the computer that runs the test.
    assert!(html.contains("value=\"all\" checked"));
    assert!(html.contains("value=\"path\" checked"));
    assert!(html.contains("Serial ports"));
}

#[tokio::test]
async fn the_settings_of_the_scanner_are_kept_per_user() {
    let server = installed(Role::Workstation, &["anna", "jan"]);
    let anna = log_in(&server.state, "anna").await;

    let body = "type=ttyUSB&type=ttyAMA&column=path&column=driver";
    let response = send(&server.state, request(Method::POST, "/scan/settings", Some(&anna), body)).await;

    // Back to the page, which then scans with the new settings.
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[LOCATION], "/scan");

    // Kept after logging in again: they are in the database of anna.
    let again = log_in(&server.state, "anna").await;
    let html = text(send(&server.state, request(Method::GET, "/scan", Some(&again), "")).await).await;

    assert!(html.contains("value=\"ttyUSB\" checked"));
    assert!(html.contains("value=\"ttyAMA\" checked"));
    assert!(!html.contains("value=\"all\" checked"));
    assert!(html.contains("value=\"driver\" checked"));
    assert!(!html.contains("value=\"type\" checked"));

    // Jan has his own settings: still the defaults.
    let jan = log_in(&server.state, "jan").await;
    let html = text(send(&server.state, request(Method::GET, "/scan", Some(&jan), "")).await).await;

    assert!(html.contains("value=\"all\" checked"));
    assert!(!html.contains("value=\"driver\" checked"));
}

#[tokio::test]
async fn the_scan_page_needs_a_login_and_a_workstation() {
    let server = installed(Role::Workstation, &["anna"]);

    let response = send(&server.state, request(Method::POST, "/scan/settings", None, "column=path")).await;
    assert_eq!(response.headers()[LOCATION], "/login");

    let master = installed(Role::Master, &["admin"]);
    let token = log_in(&master.state, "admin").await;

    for (method, path) in [(Method::GET, "/scan"), (Method::POST, "/scan/settings")] {
        let response = send(&master.state, request(method, path, Some(&token), "column=path")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

/* ---------------------------------------------------------------
 * Settings
 * ------------------------------------------------------------- */

#[tokio::test]
async fn a_user_adds_a_user_who_can_log_in() {
    let server = installed(Role::Workstation, &["anna"]);
    let token = log_in(&server.state, "anna").await;

    let body = "username=jan&password=another+good+password&password_repeat=another+good+password";
    let response = send(&server.state, request(Method::POST, "/settings/users", Some(&token), body)).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(text(response).await.contains("The user jan was added."));

    let response = send(
        &server.state,
        request(Method::POST, "/login", None, "username=jan&password=another+good+password"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let response = send(&server.state, request(Method::GET, "/settings", Some(&token), "")).await;
    let html = text(response).await;
    assert!(html.contains("<li>anna</li>") && html.contains("<li>jan</li>"));
}

#[tokio::test]
async fn a_bad_new_user_is_refused() {
    let server = installed(Role::Workstation, &["anna"]);
    let token = log_in(&server.state, "anna").await;

    for body in [
        "username=ANNA&password=another+good+password&password_repeat=another+good+password",
        "username=jan&password=short&password_repeat=short",
        "username=jan&password=another+good+password&password_repeat=different+password",
    ] {
        let response = send(&server.state, request(Method::POST, "/settings/users", Some(&token), body)).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{body}");
    }

    assert_eq!(server.state.running().unwrap().system_db.user_names().unwrap(), ["anna"]);
}

#[tokio::test]
async fn the_password_is_changed_with_the_current_one() {
    let server = installed(Role::Workstation, &["anna"]);
    let token = log_in(&server.state, "anna").await;

    let wrong = "current=not+the+password&password=a+brand+new+password&password_repeat=a+brand+new+password";
    let response = send(&server.state, request(Method::POST, "/settings/password", Some(&token), wrong)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let right = "current=correct+horse+battery&password=a+brand+new+password&password_repeat=a+brand+new+password";
    let response = send(&server.state, request(Method::POST, "/settings/password", Some(&token), right)).await;
    assert!(text(response).await.contains("Your password was changed."));

    let response = send(
        &server.state,
        request(Method::POST, "/login", None, "username=anna&password=a+brand+new+password"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn a_master_has_no_settings_page() {
    let server = installed(Role::Master, &["admin"]);
    let token = log_in(&server.state, "admin").await;

    let body = "username=mallory&password=another+good+password&password_repeat=another+good+password";

    for (method, path, body) in [(Method::GET, "/settings", ""), (Method::POST, "/settings/users", body)] {
        let response = send(&server.state, request(method, path, Some(&token), body)).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }

    assert_eq!(server.state.running().unwrap().system_db.user_names().unwrap(), ["admin"]);
}

/* ---------------------------------------------------------------
 * Files
 * ------------------------------------------------------------- */

#[tokio::test]
async fn the_style_sheet_is_served() {
    let server = installed(Role::Workstation, &["anna"]);

    let response = send(&server.state, request(Method::GET, "/static/style.css", None, "")).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers()[CONTENT_TYPE].to_str().unwrap().starts_with("text/css"));
}

#[tokio::test]
async fn unknown_addresses_are_not_found() {
    let server = installed(Role::Workstation, &["anna"]);

    let response = send(&server.state, request(Method::GET, "/no-such-page", None, "")).await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
