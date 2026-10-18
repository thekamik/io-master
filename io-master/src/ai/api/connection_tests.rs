//! Unit tests of `connection.rs`.

use super::*;

/// The template with a name and a model: a connection that passes.
fn good() -> Connection {
    Connection { name: "Local".to_owned(), model: "llama3.3:70b".to_owned(), ..Connection::template() }
}

/// The field a connection is refused for.
fn refused_field(connection: &Connection) -> Field {
    connection.check().expect_err("the connection should be refused").field
}

#[test]
fn the_template_with_a_name_and_a_model_passes() {
    let checked = good().check().unwrap();

    assert_eq!(checked.url, "http://127.0.0.1:11434/v1/chat/completions");
    assert_eq!(checked.headers, [("content-type".to_owned(), "application/json".to_owned())]);
    assert_eq!(checked.text_path.as_str(), "choices[0].delta.content");
    assert_eq!(checked.cut_reasons, ["length"]);
    assert_eq!(checked.system_role.as_deref(), Some("system"));
    assert_eq!(checked.done_marker.as_deref(), Some("[DONE]"));
    assert!(!checked.needs_key());
}

#[test]
fn a_name_is_needed() {
    let connection = Connection { name: "  ".to_owned(), ..good() };

    assert_eq!(refused_field(&connection), Field::Name);
}

#[test]
fn https_anywhere_and_http_only_to_the_loopback() {
    for url in
        ["https://ai.lab.example/v1/chat", "http://127.0.0.1:8080/x", "http://127.1.2.3/x", "http://[::1]:8000/x"]
    {
        let connection = Connection { url: url.to_owned(), ..good() };

        assert!(connection.check().is_ok(), "{url} should pass");
    }

    for url in [
        "http://localhost:11434/x",
        "http://192.168.1.10/x",
        "http://ai.lab.example/x",
        "ftp://127.0.0.1/x",
        "not a url",
        "https://user:pass@ai.lab.example/x",
        "https://ai.lab.example/x#part",
    ] {
        let connection = Connection { url: url.to_owned(), ..good() };

        assert_eq!(refused_field(&connection), Field::Url, "{url} should be refused");
    }
}

#[test]
fn the_api_key_is_refused_outside_the_headers() {
    let in_url = Connection { url: "https://ai.lab.example/x?key={{api_key}}".to_owned(), ..good() };
    let in_body = Connection { body: r#"{"messages": "{{messages}}", "key": "{{api_key}}"}"#.to_owned(), ..good() };
    let in_header = Connection { headers: "authorization: Bearer {{api_key}}".to_owned(), ..good() };

    assert_eq!(refused_field(&in_url), Field::Url);
    assert_eq!(refused_field(&in_body), Field::Body);
    assert!(in_header.check().unwrap().needs_key());
}

#[test]
fn the_model_may_be_in_the_url() {
    let connection = Connection { url: "https://ai.lab.example/models/{{model}}:generate".to_owned(), ..good() };

    let checked = connection.check().unwrap();
    assert_eq!(checked.url_with_model(), "https://ai.lab.example/models/llama3.3:70b:generate");
}

#[test]
fn a_used_model_must_have_a_name() {
    let without = Connection { model: String::new(), ..good() };
    assert_eq!(refused_field(&without), Field::Model);

    // A body without {{model}} needs none.
    let unused = Connection { model: String::new(), body: r#"{"messages": "{{messages}}"}"#.to_owned(), ..good() };
    assert!(unused.check().is_ok());

    let spaced = Connection { model: "a model".to_owned(), ..good() };
    assert_eq!(refused_field(&spaced), Field::Model);
}

#[test]
fn placeholders_in_the_body_are_whole_texts() {
    for body in [
        r#"{"model": "{{model}}"}"#,                              // no {{messages}}
        r#"{"messages": "{{messages}}", "p": "say {{system}}"}"#, // inside a text
        r#"{"messages": "{{messages}}", "x": "{{unknown}}"}"#,    // unknown
        r#"{"messages": "{{messages}}", "{{model}}": 1}"#,        // a name of a field
        r#"{"messages": "{{messages}}""#,                         // not JSON
    ] {
        let connection = Connection { body: body.to_owned(), ..good() };

        assert_eq!(refused_field(&connection), Field::Body, "{body} should be refused");
    }

    let nested = Connection {
        body: r#"{"input": {"messages": "{{messages}}", "system": ["{{system}}"]}, "model": "{{model}}"}"#.to_owned(),
        ..good()
    };
    assert!(nested.check().is_ok());
}

#[test]
fn headers_are_checked() {
    for headers in [
        "no colon here",
        "bad name: x",
        "host: other.example",
        "content-length: 5",
        "x-thing: {{messages}}",
        "x-thing: zażółć",
    ] {
        let connection = Connection { headers: headers.to_owned(), ..good() };

        assert_eq!(refused_field(&connection), Field::Headers, "{headers} should be refused");
    }

    let connection =
        Connection { headers: "\n  X-Api-Key: {{api_key}}  \n\nanthropic-version: 2023-06-01\n".to_owned(), ..good() };
    let checked = connection.check().unwrap();

    assert_eq!(
        checked.headers,
        [("x-api-key".to_owned(), "{{api_key}}".to_owned()), ("anthropic-version".to_owned(), "2023-06-01".to_owned())]
    );
}

#[test]
fn paths_are_checked_and_the_text_path_is_needed() {
    let without_text = Connection { text_path: String::new(), ..good() };
    assert_eq!(refused_field(&without_text), Field::TextPath);

    let broken_stop = Connection { stop_path: "a..b".to_owned(), ..good() };
    assert_eq!(refused_field(&broken_stop), Field::StopPath);

    let optional_empty =
        Connection { stop_path: String::new(), error_path: " ".to_owned(), done_marker: String::new(), ..good() };
    let checked = optional_empty.check().unwrap();
    assert!(checked.stop_path.is_none() && checked.error_path.is_none() && checked.done_marker.is_none());
}

#[test]
fn reasons_are_a_list() {
    let connection = Connection { cut_reasons: " max_tokens, ,model_context_window_exceeded ".to_owned(), ..good() };

    assert_eq!(connection.check().unwrap().cut_reasons, ["max_tokens", "model_context_window_exceeded"]);
}

#[test]
fn a_ca_certificate_must_be_pem() {
    let broken = Connection { ca_certificate: "not a certificate".to_owned(), ..good() };
    assert_eq!(refused_field(&broken), Field::CaCertificate);

    let key = rcgen::KeyPair::generate().unwrap();
    let pem =
        rcgen::CertificateParams::new(vec!["ai.lab.example".to_owned()]).unwrap().self_signed(&key).unwrap().pem();
    let with_ca = Connection { ca_certificate: pem, ..good() };

    assert_eq!(with_ca.check().unwrap().ca_certificates.map(|list| list.len()), Some(1));
}

#[test]
fn the_origin_is_scheme_host_and_port() {
    let connection = Connection { url: "https://ai.lab.example:8443/v1/{{model}}/chat".to_owned(), ..good() };

    assert_eq!(connection.origin().as_deref(), Some("https://ai.lab.example:8443"));
    assert_eq!(Connection { url: "nonsense".to_owned(), ..good() }.origin(), None);
}

#[test]
fn modes_have_names() {
    for mode in [AnswerMode::Stream, AnswerMode::Whole] {
        assert_eq!(AnswerMode::from_name(mode.name()), Some(mode));
    }

    assert_eq!(AnswerMode::from_name("other"), None);
}

#[test]
fn the_message_template_is_checked() {
    for template in [
        r#"{"role": "{{role}}"}"#,                            // no {{text}}
        r#"{"role": "{{role}}", "content": "say {{text}}"}"#, // inside a text
        r#"{"content": "{{text}}", "m": "{{model}}"}"#,       // not a placeholder of a message
        r#"{"content": "{{text}}""#,                          // not JSON
    ] {
        let connection = Connection { message_template: template.to_owned(), ..good() };

        assert_eq!(refused_field(&connection), Field::MessageTemplate, "{template} should be refused");
    }

    let parts = Connection {
        message_template: r#"{"role": "{{role}}", "parts": [{"text": "{{text}}"}]}"#.to_owned(),
        ..good()
    };
    assert!(parts.check().is_ok());
}

#[test]
fn every_field_is_found_by_its_name() {
    for field in Field::ALL {
        assert_eq!(Field::from_name(field.name()), Some(field));
    }

    assert_eq!(Field::from_name("api_key"), None);
}
