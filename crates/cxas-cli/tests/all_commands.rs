use std::io::Cursor;

fn run(args: &[&str]) -> (i32, serde_json::Value) {
    let mut argv = vec!["cxas".to_string()];
    argv.extend(args.iter().map(|s| s.to_string()));
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(&argv, &mut buf);
    let text = String::from_utf8(buf.into_inner()).unwrap();
    let line = text.lines().last().unwrap_or(&text);
    let v: serde_json::Value = serde_json::from_str(line).unwrap_or(serde_json::json!({
        "raw": text,
        "ok": false,
        "error": { "code": "PARSE" }
    }));
    (code, v)
}

#[test]
fn no_command_returns_not_implemented() {
    let cases: &[(&[&str], &str)] = &[
        (&["init", "--app-dir", "target/cli-init"], "init"),
        (
            &["create", "--name", "demo", "--location", "us", "--project-id", "p"],
            "create",
        ),
        (&["apps", "list", "--location", "us", "--project-id", "p"], "apps list"),
        (&["apps", "get", "--location", "us", "--app", "demo"], "apps get"),
        (
            &["push", "--app-dir", ".", "--location", "us", "--project-id", "p"],
            "push",
        ),
        (
            &[
                "branch",
                "--source",
                "demo",
                "--new-name",
                "demo2",
                "--location",
                "us",
            ],
            "branch",
        ),
        (&["conversations", "list", "--location", "us"], "conversations list"),
        (
            &[
                "conversations",
                "get",
                "--location",
                "us",
                "--conversation-resource-name",
                "c1",
            ],
            "conversations get",
        ),
        (&["deployments", "list", "--location", "us"], "deployments list"),
        (
            &[
                "deployments",
                "create",
                "--location",
                "us",
                "--deployment-id",
                "live",
            ],
            "deployments create",
        ),
        (
            &[
                "deployments",
                "promote",
                "--location",
                "us",
                "--deployment-id",
                "live",
                "--version",
                "v2",
            ],
            "deployments promote",
        ),
        (&["local", "create", "--app-dir", "target/cli-local"], "init"),
        (&["versions", "list", "--location", "us"], "versions list"),
        (&["versions", "compare", "--location", "us"], "versions compare"),
        (&["insights", "--location", "us"], "insights"),
        (&["agent", "--app-dir", "target/cli-init"], "agent"),
        (&["tool", "--app-dir", "target/cli-init"], "tool"),
        (&["guardrail", "--app-dir", "target/cli-init"], "guardrail"),
        (&["test-tools", "--test-file", "missing.yaml"], "test-tools"),
        (&["test-callbacks", "--app-dir", "."], "test-callbacks"),
        (&["export", "--evaluation-id", "e1"], "export"),
        (&["push-eval", "--file", "e.yaml"], "push-eval"),
        (&["run", "--location", "us", "--wait"], "run"),
        (&["ci-test", "--location", "us", "--app-dir", "."], "ci-test"),
        (&["local-test", "--location", "us", "--app-dir", "."], "local-test"),
        (&["delete", "--app", "demo"], "delete"),
        (
            &[
                "deploy",
                "--app-dir",
                "target/cli-init",
                "--location",
                "us",
                "--project-id",
                "p",
            ],
            "deploy",
        ),
    ];
    for (args, command) in cases {
        let (code, v) = run(args);
        let err = v["error"]["code"].as_str().unwrap_or("");
        assert_ne!(
            err, "NOT_IMPLEMENTED",
            "{command} still not implemented: {v} code={code}"
        );
        assert!(
            v["ok"].as_bool() == Some(true) || matches!(err, "USAGE" | "CES_NOT_FOUND" | "LINT_IO" | "IO" | "EVAL_FAIL" | "FEATURE_DISABLED" | "LOCATION_REQUIRED"),
            "{command} unexpected error {v} code={code}"
        );
    }
}

#[test]
fn create_then_list_round_trip() {
    let (code, created) = run(&[
        "create",
        "--name",
        "round",
        "--location",
        "europe-west1",
        "--project-id",
        "demo",
    ]);
    assert_eq!(code, 0, "{created}");
    let (code, listed) = run(&[
        "apps",
        "list",
        "--location",
        "europe-west1",
        "--project-id",
        "demo",
    ]);
    assert_eq!(code, 0, "{listed}");
    let apps = listed["data"]["apps"].as_array().unwrap();
    assert!(apps.iter().any(|a| a["display_name"] == "round"), "{listed}");
}
