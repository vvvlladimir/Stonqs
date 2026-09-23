use super::*;

#[test]
fn status_keys_match_the_typescript_types() {
    let json = serde_json::to_value(AppStatus {
        portfolio_name: "Main".into(),
        base_currency: "EUR".into(),
        inception: Some("2024-06-01".into()),
        account_count: 2,
        db_path: "/tmp/portfolio.db".into(),
        dev_build: true,
        system_locale: Some("ru-RU".into()),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "account_count",
            "base_currency",
            "db_path",
            "dev_build",
            "inception",
            "portfolio_name",
            "system_locale"
        ]
    );
}

#[test]
fn error_codes_match_the_typescript_union() {
    let error = sq_core::Error::MissingMarketData {
        kind: "quote",
        key: "AAPL".into(),
        date: chrono::NaiveDate::from_ymd_opt(2024, 6, 5).unwrap(),
    };
    let json = serde_json::to_value(sq_app_lib::error::UiError::from(error)).unwrap();

    // The frontend switches on the stable error code.
    assert_eq!(json["code"], "missing_market_data");
    assert_eq!(keys(&json), ["code", "date", "key", "kind", "message"]);
}

/// Progress events are discriminated by the `event` field.
#[test]
fn job_progress_is_tagged_by_event() {
    use sq_app_lib::jobs::{Failure, FailureCause, FailureCode, Progress};

    let json = serde_json::to_value(Progress::Started { total: 3 }).unwrap();
    assert_eq!(json["event"], "started");

    let json = serde_json::to_value(Progress::Item {
        label: "AAPL".into(),
        done: 1,
        total: 3,
        fetched: 250,
    })
    .unwrap();
    assert_eq!(json["event"], "item");
    assert_eq!(keys(&json), ["done", "event", "fetched", "label", "total"]);

    let json = serde_json::to_value(Progress::Finished {
        fetched: 250,
        failed: vec![Failure {
            code: FailureCode::Quote,
            cause: FailureCause::NoData,
            subject: "IWDA".into(),
            detail: "network error".into(),
            source: Some("yahoo".into()),
        }],
        cancelled: false,
    })
    .unwrap();
    assert_eq!(json["event"], "finished");
    assert_eq!(keys(&json), ["cancelled", "event", "failed", "fetched"]);
    assert_eq!(json["failed"][0]["code"], "quote");
    assert_eq!(json["failed"][0]["subject"], "IWDA");
    assert_eq!(json["failed"][0]["cause"], "no_data");
    assert_eq!(json["failed"][0]["source"], "yahoo");
    assert_eq!(
        keys(&json["failed"][0]),
        ["cause", "code", "detail", "source", "subject"]
    );
}

/// Settings written by an older build carry no language; it must read back as "system".
#[test]
fn settings_keys_match_the_typescript_types() {
    use sq_app_lib::settings::AppSettings;

    let json = serde_json::to_value(AppSettings::default()).unwrap();
    assert_eq!(
        keys(&json),
        [
            "ai_custom",
            "ai_effort",
            "ai_enabled",
            "ai_extra_models",
            "ai_models",
            "ai_provider",
            "ai_reasoning",
            "ai_web_search",
            "auto_refresh_on_start",
            "hidden_presets",
            "language",
            "last_refresh",
            "market_custom",
            "market_sources",
            "periods",
            "refresh_min_interval_hours",
            "scope",
            "ui"
        ]
    );
    assert_eq!(json["language"], "system");
    assert_eq!(json["ai_enabled"], false);
    assert_eq!(json["ai_provider"], "openai");
    // No model is chosen up front: a chat lands on its provider's smallest tier today until the
    // user picks one in a chat, which is then remembered per provider (ADR-0069).
    assert_eq!(json["ai_models"], serde_json::json!({}));
    assert_eq!(json["ai_effort"], "MEDIUM");
    assert_eq!(keys(&json["ai_custom"]), ["base_url", "label", "model", "wire"]);
    assert_eq!(json["ai_custom"]["wire"], "OPENAI_CHAT");

    let old: AppSettings = serde_json::from_str(
        r#"{"auto_refresh_on_start":true,"refresh_min_interval_hours":6,"last_refresh":null}"#,
    )
    .unwrap();
    assert_eq!(old.language, "system");
    assert!(
        old.periods.is_empty(),
        "an older file has no period axis of its own"
    );
    assert!(old.hidden_presets.is_empty());
    assert!(
        !old.ai_enabled,
        "an older file has no AI panel, so it must not turn on by itself"
    );
    assert_eq!(old.ai_provider, "openai");
    assert!(old.ai_models.is_empty());
}

/// A user period's spec is tagged, so the two kinds are told apart by a field, not by shape.
#[test]
fn user_period_specs_carry_their_kind() {
    use sq_app_lib::commands::periods::PeriodRange;
    use sq_app_lib::settings::UserPeriod;
    use sq_core::calc::{Period, PeriodSpec};

    let relative = serde_json::to_value(UserPeriod {
        id: "p-1".into(),
        name: "Six months".into(),
        spec: PeriodSpec::Relative {
            unit: Period::Month,
            count: 6,
        },
    })
    .unwrap();
    assert_eq!(keys(&relative), ["id", "name", "spec"]);
    assert_eq!(keys(&relative["spec"]), ["count", "kind", "unit"]);
    assert_eq!(relative["spec"]["kind"], "RELATIVE");
    assert_eq!(relative["spec"]["unit"], "MONTH");
    assert!(relative["spec"]["count"].is_number());

    let fixed = serde_json::to_value(UserPeriod {
        id: "p-2".into(),
        name: "2024".into(),
        spec: PeriodSpec::Fixed {
            from: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            to: Some(chrono::NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
        },
    })
    .unwrap();
    assert_eq!(keys(&fixed["spec"]), ["from", "kind", "to"]);
    assert_eq!(fixed["spec"]["kind"], "FIXED");
    assert_eq!(fixed["spec"]["from"], "2024-01-01");

    // An open end travels as null and reads back from a file that never wrote the key at all.
    let open = serde_json::to_value(PeriodSpec::Fixed {
        from: chrono::NaiveDate::from_ymd_opt(2024, 5, 12).unwrap(),
        to: None,
    })
    .unwrap();
    assert_eq!(open["to"], Value::Null);
    let parsed: PeriodSpec = serde_json::from_str(r#"{"kind":"FIXED","from":"2024-05-12"}"#).unwrap();
    assert_eq!(
        parsed,
        PeriodSpec::Fixed {
            from: chrono::NaiveDate::from_ymd_opt(2024, 5, 12).unwrap(),
            to: None,
        }
    );

    // A shipped preset carries no name: its wording belongs to the frontend (ADR-0023).
    let shipped = serde_json::to_value(PeriodRange {
        id: "ONE_MONTH".into(),
        name: None,
        from: "2024-08-14".into(),
        to: "2024-09-14".into(),
    })
    .unwrap();
    assert_eq!(keys(&shipped), ["from", "id", "name", "to"]);
    assert_eq!(shipped["name"], Value::Null);
}

/// Refresh is due when the timestamp is missing or invalid.
#[test]
fn auto_refresh_respects_the_interval() {
    use sq_app_lib::settings::AppSettings;

    let now = chrono::Utc::now();
    let base = AppSettings {
        auto_refresh_on_start: true,
        refresh_min_interval_hours: 6,
        last_refresh: None,
        scope: Default::default(),
        language: "system".into(),
        periods: Vec::new(),
        hidden_presets: Vec::new(),
        ui: serde_json::Value::Null,
        ai_enabled: false,
        ai_provider: "openai".into(),
        ai_models: Default::default(),
        ai_extra_models: Default::default(),
        ai_effort: Default::default(),
        ai_web_search: true,
        ai_reasoning: false,
        ai_custom: Default::default(),
        market_sources: Default::default(),
        market_custom: Vec::new(),
    };

    assert!(base.due(now), "no timestamp means the data is due");

    let recent = AppSettings {
        last_refresh: Some((now - chrono::Duration::hours(1)).to_rfc3339()),
        ..base.clone()
    };
    assert!(!recent.due(now), "an hour ago is too early");

    let stale = AppSettings {
        last_refresh: Some((now - chrono::Duration::hours(7)).to_rfc3339()),
        ..base.clone()
    };
    assert!(stale.due(now), "seven hours ago is due");

    let broken = AppSettings {
        last_refresh: Some("yesterday".into()),
        ..base.clone()
    };
    assert!(broken.due(now), "a broken timestamp must not block the refresh");

    let disabled = AppSettings {
        auto_refresh_on_start: false,
        ..base
    };
    assert!(!disabled.due(now), "off means off");
}

/// Scope state uses SCREAMING_SNAKE_CASE for `kind`.
#[test]
fn data_scope_crosses_with_a_stable_shape() {
    use sq_app_lib::scope::{DataScope, ScopeKind};

    let json = serde_json::to_value(DataScope::portfolio()).unwrap();
    assert_eq!(keys(&json), ["id", "kind"]);
    assert_eq!(json["kind"], "PORTFOLIO");
    assert_eq!(json["id"], Value::Null);

    let group = serde_json::to_value(DataScope {
        kind: ScopeKind::Group,
        id: Some("g-1".into()),
    })
    .unwrap();
    assert_eq!(group["kind"], "GROUP");
    assert_eq!(group["id"], "g-1");
}

/// A group is intersected with the portfolio's active accounts.
#[test]
fn scope_intersects_the_portfolio_instead_of_replacing_it() {
    use sq_app_lib::scope::{DataScope, ScopeKind};
    use sq_core::model::{AccountGroup, Portfolio};

    let portfolio = Portfolio::new("Main", "EUR").with_accounts(["a".to_string(), "b".to_string()]);
    let group = AccountGroup {
        id: "g-1".into(),
        name: "Pension".into(),
        // Account "c" is outside the portfolio.
        account_ids: vec!["b".into(), "c".into()],
    };

    let scoped = DataScope {
        kind: ScopeKind::Group,
        id: Some("g-1".into()),
    }
    .apply(&portfolio, std::slice::from_ref(&group), &[]);
    assert_eq!(scoped.account_ids, vec!["b".to_string()]);
    assert_eq!(
        scoped.base_currency, "EUR",
        "the reporting currency does not depend on the scope"
    );

    // A missing group falls back to the full portfolio.
    let missing = DataScope {
        kind: ScopeKind::Group,
        id: Some("gone".into()),
    }
    .apply(&portfolio, &[group], &[]);
    assert_eq!(missing.account_ids.len(), 2);

    let one = DataScope {
        kind: ScopeKind::Account,
        id: Some("a".into()),
    }
    .apply(&portfolio, &[], &[]);
    assert_eq!(one.account_ids, vec!["a".to_string()]);
}

/// The profile list the picker, the lock screen and the Profiles panel read.
#[test]
fn profile_list_keys_match_the_typescript_types() {
    use sq_app_lib::commands::profiles::{ProfileList, ProfileView};
    let json = serde_json::to_value(ProfileList {
        profiles: vec![ProfileView {
            profile: sq_app_lib::profiles::Profile {
                id: "default".into(),
                name: "Default".into(),
                created_at: "2026-09-19T10:00:00+00:00".into(),
            },
            protected: true,
        }],
        open: "default".into(),
        locked: true,
        remembered: false,
    })
    .unwrap();
    assert_eq!(keys(&json), ["locked", "open", "profiles", "remembered"]);
    assert_eq!(
        keys(&json["profiles"][0]),
        ["created_at", "id", "name", "protected"]
    );
}

#[test]
fn a_plugin_carries_its_status_flattened_beside_its_name() {
    use sq_app_lib::plugins::{Base, PluginInfo, Status, ThemeDef};

    let json = serde_json::to_value(PluginInfo {
        id: "com.example.midnight".into(),
        name: "Midnight".into(),
        version: "1.0.0".into(),
        themes: vec![ThemeDef {
            id: "midnight".into(),
            name: "Midnight".into(),
            file: "midnight.css".into(),
            base: Base::Dark,
        }],
        status: Status::Api { wants: 2, speaks: 1 },
    })
    .unwrap();

    // The status is a discriminated union on `status`, the way `UiError` is on `code`: the
    // frontend writes the sentence, the host says which one and with what values.
    assert_eq!(
        keys(&json),
        ["id", "name", "speaks", "status", "themes", "version", "wants"]
    );
    assert_eq!(json["status"], "api");
    assert_eq!(json["themes"][0]["base"], "dark");
}
