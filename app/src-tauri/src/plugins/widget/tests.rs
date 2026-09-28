use super::*;

fn def() -> WidgetDef {
    WidgetDef {
        id: "heat".into(),
        name: "Heat".into(),
        description: String::new(),
        file: "heat.js".into(),
        reads: vec![Read::Positions],
        periodic: false,
        size: Size { w: 6, h: 8 },
        min: Size { w: 3, h: 4 },
    }
}

#[test]
fn the_page_allows_no_request_of_any_kind() {
    let (_, csp) = page("", "n0");
    assert!(csp.starts_with("default-src 'none'"));
    assert!(!csp.contains("connect-src"), "no fetch, no socket, no IPC scheme");
    assert!(
        csp.contains("script-src 'nonce-n0'"),
        "no script but the page's own"
    );
}

/// The two ways out a content policy does not cover: a peer connection and a DNS lookup.
#[test]
fn the_page_closes_what_the_policy_cannot() {
    let (html, _) = page("", "n0");
    let shim = html.find("RTC").expect("the shim removes the peer connection");
    let module = html.find("<script type=\"module\"").unwrap();
    assert!(shim < module, "removed before the module can take a copy");
    assert!(html.contains("x-dns-prefetch-control\" content=\"off\""));
}

#[test]
fn a_closing_script_tag_in_any_case_does_not_end_the_element() {
    let (html, _) = page("const s = \"</SCRIPT><script>alert(1)\"; // </Script>", "n0");
    assert_eq!(
        html.to_lowercase().matches("</script>").count(),
        2,
        "the shim's and the module's own"
    );
    assert!(
        html.contains("<\\/SCRIPT>") && html.contains("<\\/Script>"),
        "case kept"
    );
}

#[test]
fn a_closing_script_tag_in_the_module_does_not_end_the_element() {
    let (html, _) = page("const s = \"</script><script>alert(1)\";", "n0");
    assert!(!html.contains("\"</script><script>alert"));
    assert_eq!(
        html.matches("</script>").count(),
        2,
        "the shim's and the module's own"
    );
}

#[test]
fn a_widget_smaller_than_its_minimum_is_refused() {
    assert!(check(&def(), b"export {}").is_ok());
    let bad = WidgetDef {
        min: Size { w: 8, h: 4 },
        ..def()
    };
    assert!(check(&bad, b"").is_err());
    let wide = WidgetDef {
        size: Size { w: 13, h: 8 },
        ..def()
    };
    assert!(check(&wide, b"").is_err());
}

#[test]
fn performance_needs_a_period() {
    let bad = WidgetDef {
        reads: vec![Read::Performance],
        ..def()
    };
    assert!(check(&bad, b"").is_err());
    assert!(
        check(
            &WidgetDef {
                periodic: true,
                ..bad
            },
            b""
        )
        .is_ok()
    );
}

#[test]
fn transactions_are_a_screens_read_and_need_a_period() {
    let widget = WidgetDef {
        reads: vec![Read::Transactions],
        periodic: true,
        ..def()
    };
    assert!(
        check(&widget, b"").is_err(),
        "a widget has a source the operation list ignores"
    );

    let screen = ScreenDef {
        id: "spending".into(),
        name: "Spending".into(),
        description: String::new(),
        file: "spending.js".into(),
        reads: vec![Read::Transactions],
        periodic: false,
        storage: true,
    };
    assert!(check_screen(&screen, b"").is_err());
    assert!(
        check_screen(
            &ScreenDef {
                periodic: true,
                ..screen
            },
            b""
        )
        .is_ok()
    );
}
