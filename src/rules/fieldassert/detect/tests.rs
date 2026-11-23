// standard crates
use std::io::Write;
use std::path::Path;

// internal crates
use crate::rules::{fieldassert::detect::*, Limits};

// external crates
use tempfile::NamedTempFile;

fn write_temp_file(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn check_file(path: &Path, source: &str, threshold: usize) -> Vec<Diagnostic> {
    let ast = syn::parse_file(source).unwrap();
    let limits = Limits {
        funclen: 0,
        field_assert: threshold,
        filelen: 0,
    };
    check(&Input {
        path,
        source,
        ast: &ast,
        limits: &limits,
    })
}

#[test]
fn test_flags_four_field_asserts() {
    let src = r#"
#[test]
fn test_example() {
    let req = get_request();
    assert_eq!(req.call, Call::GetDeployment);
    assert_eq!(req.method, Method::GET);
    assert_eq!(req.path, "/deployments");
    assert_eq!(req.url, "http://mock/deployments");
}
"#;
    let f = write_temp_file(src);
    let violations = check_file(f.path(), src, 4);
    assert_eq!(violations.len(), 1);
    assert_eq!(
        violations[0].message,
        "4 assert_eq! calls on fields of `req` \u{2014} \
         consider constructing an expected struct"
    );
}

#[test]
fn test_below_threshold_no_violation() {
    let src = r#"
#[test]
fn test_example() {
    let req = get_request();
    assert_eq!(req.call, Call::GetDeployment);
    assert_eq!(req.method, Method::GET);
    assert_eq!(req.path, "/deployments");
}
"#;
    let f = write_temp_file(src);
    let violations = check_file(f.path(), src, 4);
    assert!(violations.is_empty());
}

#[test]
fn test_different_receivers_not_grouped() {
    let src = r#"
#[test]
fn test_example() {
    assert_eq!(req.call, Call::GetDeployment);
    assert_eq!(req.method, Method::GET);
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, "ok");
}
"#;
    let f = write_temp_file(src);
    let violations = check_file(f.path(), src, 4);
    assert!(violations.is_empty());
}

#[test]
fn test_escape_hatch_suppresses() {
    let src = r#"
#[test]
fn test_example() {
    // lint:allow(field-by-field-assert)
    let req = get_request();
    assert_eq!(req.call, Call::GetDeployment);
    assert_eq!(req.method, Method::GET);
    assert_eq!(req.path, "/deployments");
    assert_eq!(req.url, "http://mock/deployments");
    assert_eq!(req.body, "{}");
}
"#;
    let f = write_temp_file(src);
    let violations = check_file(f.path(), src, 4);
    assert!(violations.is_empty());
}

#[test]
fn test_non_test_function_ignored() {
    let src = r#"
fn helper() {
    assert_eq!(req.call, Call::GetDeployment);
    assert_eq!(req.method, Method::GET);
    assert_eq!(req.path, "/deployments");
    assert_eq!(req.url, "http://mock/deployments");
    assert_eq!(req.body, "{}");
}
"#;
    let f = write_temp_file(src);
    let violations = check_file(f.path(), src, 4);
    assert!(violations.is_empty());
}

#[test]
fn test_tokio_test_detected() {
    let src = r#"
#[tokio::test]
async fn test_async_example() {
    let req = get_request().await;
    assert_eq!(req.call, Call::GetDeployment);
    assert_eq!(req.method, Method::GET);
    assert_eq!(req.path, "/deployments");
    assert_eq!(req.url, "http://mock/deployments");
}
"#;
    let f = write_temp_file(src);
    let violations = check_file(f.path(), src, 4);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].message.contains("`req`"));
}

#[test]
fn test_nested_mod_tests() {
    let src = r#"
mod tests {
    #[test]
    fn test_nested() {
        assert_eq!(req.call, Call::GetDeployment);
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.path, "/deployments");
        assert_eq!(req.url, "http://mock/deployments");
    }
}
"#;
    let f = write_temp_file(src);
    let violations = check_file(f.path(), src, 4);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 5);
}

#[test]
fn test_custom_threshold() {
    let src = r#"
#[test]
fn test_example() {
    assert_eq!(req.call, Call::GetDeployment);
    assert_eq!(req.method, Method::GET);
}
"#;
    let f = write_temp_file(src);
    let violations = check_file(f.path(), src, 2);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].message.starts_with("2 assert_eq!"));
}

#[test]
fn test_is_test_fn_plain() {
    let item: syn::ItemFn = syn::parse_str(
        r#"
            #[test]
            fn my_test() {}
            "#,
    )
    .unwrap();
    assert!(is_test_fn(&item));
}

#[test]
fn test_is_test_fn_tokio() {
    let item: syn::ItemFn = syn::parse_str(
        r#"
            #[tokio::test]
            async fn my_test() {}
            "#,
    )
    .unwrap();
    assert!(is_test_fn(&item));
}

#[test]
fn test_is_not_test_fn() {
    let item: syn::ItemFn = syn::parse_str(
        r#"
            fn helper() {}
            "#,
    )
    .unwrap();
    assert!(!is_test_fn(&item));
}

#[test]
fn test_parse_first_arg_simple() {
    let tokens: TokenStream = "req.call, Call::GetDeployment".parse().unwrap();
    let expr = parse_first_arg(&tokens).unwrap();
    assert_eq!(root_receiver(&expr), Some("req".to_string()));
}

#[test]
fn test_parse_first_arg_with_nested_comma() {
    // The comma inside the function call should not split.
    let tokens: TokenStream = "req.items, vec![1, 2, 3]".parse().unwrap();
    let expr = parse_first_arg(&tokens).unwrap();
    assert_eq!(root_receiver(&expr), Some("req".to_string()));
}
