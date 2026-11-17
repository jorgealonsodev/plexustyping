use super::*;

fn attempt() -> Attempt {
    Attempt {
        state: "st".into(),
        nonce: "no".into(),
        verifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into(),
    }
}

#[test]
fn pkce_challenge_matches_rfc_7636_example() {
    assert_eq!(
        pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[test]
fn random_tokens_are_32_bytes_of_base64url() {
    let a = random_token();
    assert_eq!(a.len(), 43);
    assert!(
        a.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    );
    assert_ne!(a, random_token());
}

#[test]
fn host_ids_are_uuid_v4_urns() {
    let id = new_host_id();
    assert_eq!(id.len(), "urn:uuid:".len() + 36);
    assert!(id.starts_with("urn:uuid:"));
    assert_eq!(&id[9 + 14..9 + 15], "4");
    assert!(matches!(&id[9 + 19..9 + 20], "8" | "9" | "a" | "b"));
    assert_ne!(id, new_host_id());
}

#[test]
fn queries_decode_percent_and_plus_and_keep_the_first_repeat() {
    let q = parse_query("code=a%2Fb&scope=openid+email&state=x&empty=&state=y");
    assert_eq!(q["code"], "a/b");
    assert_eq!(q["scope"], "openid email");
    assert_eq!(q["empty"], "");
    assert_eq!(q["state"], "x");
}

#[test]
fn first_authorization_registers_with_the_dynamic_client() {
    let record = Record {
        host_id: "urn:uuid:h".into(),
        ..Record::default()
    };
    let url = authorize_url(
        &Endpoints::production(),
        &record,
        None,
        "http://127.0.0.1:1455/auth/callback",
        &attempt(),
    );
    let (base, query) = url.split_once('?').unwrap();
    assert_eq!(base, "https://auth.openai.com/api/accounts/authorize");
    let q = parse_query(query);
    let mut keys: Vec<&str> = q.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "agent_name_hint",
            "client_id",
            "code_challenge",
            "code_challenge_method",
            "ext_agent_host_id",
            "nonce",
            "redirect_uri",
            "resource",
            "response_type",
            "scope",
            "state"
        ]
    );
    assert_eq!(q["response_type"], "code");
    assert_eq!(q["client_id"], "dynamic_agent_client");
    assert_eq!(q["agent_name_hint"], "PlexusTyping");
    assert_eq!(q["ext_agent_host_id"], "urn:uuid:h");
    assert_eq!(q["redirect_uri"], "http://127.0.0.1:1455/auth/callback");
    assert_eq!(
        q["scope"],
        "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct"
    );
    assert_eq!(q["resource"], "https://api.openai.com/v1");
    assert_eq!(q["state"], "st");
    assert_eq!(q["nonce"], "no");
    assert_eq!(q["code_challenge_method"], "S256");
    assert_eq!(
        q["code_challenge"],
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[test]
fn reauthorization_uses_the_issued_client_and_both_hints() {
    let record = Record {
        host_id: "urn:uuid:h".into(),
        client_id: Some("oaiapp_1".into()),
        email: Some("a@b.c".into()),
        id_token: Some("old.id.token".into()),
        ..Record::default()
    };
    let url = authorize_url(
        &Endpoints::production(),
        &record,
        Some("oaiapp_1"),
        "http://127.0.0.1:9/auth/callback",
        &attempt(),
    );
    let q = parse_query(url.split_once('?').unwrap().1);
    assert_eq!(q["client_id"], "oaiapp_1");
    assert!(!q.contains_key("agent_name_hint"));
    assert_eq!(q["id_token_hint"], "old.id.token");
    assert_eq!(q["login_hint"], "a@b.c");
    assert_eq!(q["ext_agent_host_id"], "urn:uuid:h");
}

#[test]
fn a_retained_client_of_another_registration_gets_no_hints() {
    let record = Record {
        host_id: "urn:uuid:h".into(),
        email: Some("a@b.c".into()),
        ..Record::default()
    };
    let url = authorize_url(
        &Endpoints::production(),
        &record,
        Some("oaiapp_new"),
        "http://127.0.0.1:9/auth/callback",
        &attempt(),
    );
    let q = parse_query(url.split_once('?').unwrap().1);
    assert_eq!(q["client_id"], "oaiapp_new");
    assert!(!q.contains_key("agent_name_hint"));
    assert!(!q.contains_key("login_hint"));
    assert!(!q.contains_key("id_token_hint"));
}

#[test]
fn issued_client_ids_are_checked() {
    assert!(valid_client_id("oaiapp_AbC-1.2"));
    assert!(!valid_client_id("dynamic_agent_client"));
    assert!(!valid_client_id(""));
    assert!(!valid_client_id("a b"));
    assert!(!valid_client_id(&"a".repeat(129)));
}

#[test]
fn oauth_errors_keep_only_a_safe_code() {
    let body = br#"{"error":"invalid_grant","error_description":"rt-SECRET-x"}"#;
    assert_eq!(oauth_error(body).as_deref(), Some("invalid_grant"));
    assert_eq!(
        oauth_error(br#"{"error":{"code":"bad code <x>"}}"#).as_deref(),
        Some("badcodex")
    );
    assert_eq!(oauth_error(b"{}"), None);
    assert_eq!(oauth_error(b"not json"), None);
    assert_eq!(safe_code(&"a".repeat(80)).len(), 64);
}

#[test]
fn the_record_debug_output_has_no_tokens() {
    let record = Record {
        access_token: Some("secret-access".into()),
        refresh_token: Some("secret-refresh".into()),
        id_token: Some("secret-id".into()),
        ..Record::default()
    };
    let debug = format!("{record:?} {record:#?}");
    assert!(!debug.contains("secret"), "{debug}");
}

#[test]
fn origins_compare_scheme_host_and_port() {
    assert_eq!(
        origin("https://auth.openai.com/oauth/revoke"),
        Some("https://auth.openai.com")
    );
    assert_eq!(origin("http://127.0.0.1:9/x"), Some("http://127.0.0.1:9"));
    assert_eq!(
        origin("https://auth.openai.com"),
        Some("https://auth.openai.com")
    );
    assert_eq!(origin("nope"), None);
}

#[test]
fn defaults_follow_the_docs() {
    let auth = ChatgptAuth::new(&Endpoints::production(), None);
    assert_eq!(auth.callback_port(), 1455);
    assert_eq!(auth.sign_in_timeout(), Duration::from_secs(300));
    assert_eq!(CREDENTIALS_FILE, "chatgpt-auth.json");
    assert_eq!(auth_base(&auth.endpoints), "https://auth.openai.com");
    assert_eq!(
        token_url(&auth.endpoints),
        "https://auth.openai.com/api/accounts/oauth/token"
    );
}
