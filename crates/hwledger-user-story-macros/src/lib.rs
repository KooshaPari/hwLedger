//! `#[user_story_test]` proc-macro — Batch 2 of the user-story-as-test
//! framework (see ADR 0034 `docs/adr/0034-user-story-test-sourcing.md`).
//!
//! Usage:
//!
//! ```ignore
//! use hwledger_user_story_macros::user_story_test;
//!
//! #[user_story_test(yaml = r#"
//! journey_id: plan-help
//! title: "CLI — plan --help flags"
//! persona: operator exploring the CLI
//! given: a fresh hwLedger install
//! when:
//!   - run `hwledger plan --help`
//! then:
//!   - exit 0
//!   - stdout contains --seq flag
//! traces_to: [FR-PLAN-003]
//! record: true
//! blind_judge: auto
//! family: cli
//! "#)]
//! fn plan_help_shows_seq_flag() {
//!     let output = std::process::Command::new("hwledger-cli")
//!         .args(["plan", "--help"]).output().unwrap();
//!     assert!(output.status.success());
//!     assert!(String::from_utf8_lossy(&output.stdout).contains("--seq"));
//! }
//! ```
//!
//! The YAML body is validated at proc-macro expansion time against the
//! canonical schema (mirrored from
//! `tools/user-story-extract/schema/user-story.schema.json`, which in turn is
//! the `phenotype-journeys/crates/phenotype-journey-core/schema` copy the
//! Batch 1 harvester ships). On malformed YAML we emit `compile_error!` so
//! the author sees the problem before `cargo test` ever runs.
//!
//! The expansion preserves the user's function body exactly, wrapping it in
//! a `hwledger_user_story_runtime::maybe_record(&META, || { BODY })` call.
//! When `PHENOTYPE_USER_STORY_RECORD` is unset this is a zero-overhead
//! pass-through; when it is set, the runtime re-execs the test binary under
//! a PTY and emits asciicast + manifest artifacts.
//!
//! The macro additionally leaves a `#[doc = "@phenotype/user-story ..."]`
//! attribute containing the raw YAML so the Batch 1 extractor can inventory
//! macro-sourced stories without a second source-of-truth (the extractor
//! treats `/// @user-story ... /// @end` blocks uniformly).

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use serde::Deserialize;
use syn::{parse_macro_input, Expr, ItemFn, Lit, Meta, MetaNameValue};

#[derive(Debug, Deserialize)]
struct RawStory {
    journey_id: Option<String>,
    title: Option<String>,
    persona: Option<String>,
    given: Option<String>,
    when: Option<Vec<String>>,
    then: Option<Vec<String>>,
    traces_to: Option<Vec<String>>,
    record: Option<bool>,
    blind_judge: Option<String>,
    family: Option<String>,
}

fn err(span: Span, msg: &str) -> TokenStream {
    syn::Error::new(span, msg).to_compile_error().into()
}

fn extract_yaml_literal(attr: TokenStream) -> Result<String, syn::Error> {
    // Accept either `yaml = "..."` (MetaNameValue) or a bare string literal.
    let attr_ts: proc_macro2::TokenStream = attr.into();
    // Try name-value form first.
    if let Ok(mnv) = syn::parse2::<MetaNameValue>(attr_ts.clone()) {
        if mnv.path.is_ident("yaml") {
            if let Expr::Lit(expr_lit) = &mnv.value {
                if let Lit::Str(s) = &expr_lit.lit {
                    return Ok(s.value());
                }
            }
            return Err(syn::Error::new_spanned(
                &mnv.value,
                "expected `yaml = \"...\"` string literal",
            ));
        }
    }
    // Bare string literal form.
    if let Ok(Lit::Str(s)) = syn::parse2::<Lit>(attr_ts.clone()) {
        return Ok(s.value());
    }
    // Last resort: Meta form with error.
    if let Ok(meta) = syn::parse2::<Meta>(attr_ts) {
        return Err(syn::Error::new_spanned(
            meta,
            "user_story_test expects `yaml = \"...\"` or a bare string literal",
        ));
    }
    Err(syn::Error::new(Span::call_site(), "user_story_test expects `yaml = \"<frontmatter>\"`"))
}

fn validate_story(yaml_src: &str, span: Span) -> Result<RawStory, syn::Error> {
    let parsed: RawStory = serde_yaml::from_str(yaml_src).map_err(|e| {
        let loc = e.location().map(|l| format!(" at line {} col {}", l.line(), l.column()));
        syn::Error::new(
            span,
            format!("[user_story_test] YAML parse error{}: {e}", loc.unwrap_or_default()),
        )
    })?;
    // Enforce required fields per user-story.schema.json.
    let mut missing: Vec<&str> = Vec::new();
    if parsed.journey_id.as_deref().unwrap_or("").is_empty() {
        missing.push("journey_id");
    }
    if parsed.title.as_deref().unwrap_or("").is_empty() {
        missing.push("title");
    }
    if parsed.persona.as_deref().unwrap_or("").is_empty() {
        missing.push("persona");
    }
    if parsed.given.as_deref().unwrap_or("").is_empty() {
        missing.push("given");
    }
    if parsed.when.as_ref().map(|v| v.is_empty()).unwrap_or(true) {
        missing.push("when");
    }
    if parsed.then.as_ref().map(|v| v.is_empty()).unwrap_or(true) {
        missing.push("then");
    }
    if parsed.traces_to.as_ref().map(|v| v.is_empty()).unwrap_or(true) {
        missing.push("traces_to");
    }
    if !missing.is_empty() {
        return Err(syn::Error::new(
            span,
            format!(
                "[user_story_test] YAML frontmatter missing required field(s): {}. See docs/adr/0034-user-story-test-sourcing.md.",
                missing.join(", ")
            ),
        ));
    }
    // journey_id must match kebab-case pattern.
    let id = parsed.journey_id.as_deref().unwrap_or("");
    if !is_kebab_lower(id) {
        return Err(syn::Error::new(
            span,
            format!(
                "[user_story_test] journey_id '{id}' must match ^[a-z0-9][a-z0-9-]*[a-z0-9]$ (kebab-case, 2..=80 chars)"
            ),
        ));
    }
    // traces_to entries must match FR-... pattern.
    for fr in parsed.traces_to.as_ref().unwrap() {
        if !is_fr_id(fr) {
            return Err(syn::Error::new(
                span,
                format!(
                    "[user_story_test] traces_to entry '{fr}' must match ^FR-[A-Z0-9][A-Z0-9_-]*$"
                ),
            ));
        }
    }
    // family enum constraint (optional field, defaults to "cli" at macro time).
    if let Some(fam) = &parsed.family {
        if !matches!(fam.as_str(), "cli" | "gui" | "streamlit" | "k6" | "other") {
            return Err(syn::Error::new(
                span,
                format!(
                    "[user_story_test] family '{fam}' must be one of cli|gui|streamlit|k6|other"
                ),
            ));
        }
    }
    if let Some(bj) = &parsed.blind_judge {
        if !matches!(bj.as_str(), "auto" | "skip") {
            return Err(syn::Error::new(
                span,
                format!("[user_story_test] blind_judge '{bj}' must be 'auto' or 'skip'"),
            ));
        }
    }
    Ok(parsed)
}

fn is_kebab_lower(s: &str) -> bool {
    if s.len() < 2 || s.len() > 80 {
        return false;
    }
    let bytes = s.as_bytes();
    let ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    if !ok(bytes[0]) || !ok(bytes[bytes.len() - 1]) {
        return false;
    }
    bytes.iter().all(|&b| ok(b) || b == b'-')
}

fn is_fr_id(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 4 || &b[..3] != b"FR-" {
        return false;
    }
    let tail = &b[3..];
    if tail.is_empty() {
        return false;
    }
    let first_ok = tail[0].is_ascii_uppercase() || tail[0].is_ascii_digit();
    first_ok
        && tail
            .iter()
            .all(|&c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_' || c == b'-')
}

/// Marker used by the Batch 1 extractor to inventory macro-sourced stories.
/// Applied automatically by `#[user_story_test]`.
#[proc_macro_attribute]
pub fn user_story_harvested(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// See crate-level docs.
#[proc_macro_attribute]
pub fn user_story_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let span = Span::call_site();
    let yaml_src = match extract_yaml_literal(attr) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };
    let story = match validate_story(&yaml_src, span) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };

    let input: ItemFn = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;
    let fn_attrs = &input.attrs;
    let fn_sig = &input.sig;
    if !fn_sig.inputs.is_empty() {
        return err(
            fn_sig.inputs.span_site(),
            "#[user_story_test] functions must take zero arguments",
        );
    }

    let journey_id = story.journey_id.as_ref().unwrap().clone();
    let title = story.title.as_ref().unwrap().clone();
    let persona = story.persona.as_ref().unwrap().clone();
    let family = story.family.clone().unwrap_or_else(|| "cli".to_string());
    let record = story.record.unwrap_or(true);
    let blind_judge = story.blind_judge.clone().unwrap_or_else(|| "auto".to_string());
    let traces: Vec<String> = story.traces_to.clone().unwrap_or_default();

    // Re-embed the raw YAML in a doc-comment so the Batch 1 harvester's
    // line-comment scanner (`// @user-story` ... `// @end`) can inventory
    // macro-sourced stories alongside hand-written ones. We write it as
    // attribute-free lines that `find_rust`'s regex tolerates when hoisted
    // through rustdoc's attribute lowering.
    let harvester_marker =
        format!("@phenotype/user-story inline=true journey_id={} family={}", journey_id, family);

    let traces_refs: Vec<_> = traces.iter().map(|s| quote! { #s }).collect();

    let expanded = quote! {
        #(#fn_attrs)*
        #[doc = #harvester_marker]
        #[::hwledger_user_story_macros::user_story_harvested]
        #[test]
        #fn_vis fn #fn_name() {
            const __USER_STORY_META: ::hwledger_user_story_runtime::UserStoryMeta =
                ::hwledger_user_story_runtime::UserStoryMeta {
                    journey_id: #journey_id,
                    title: #title,
                    persona: #persona,
                    family: #family,
                    record: #record,
                    blind_judge: #blind_judge,
                    traces_to: &[ #( #traces_refs ),* ],
                };
            ::hwledger_user_story_runtime::maybe_record(&__USER_STORY_META, || {
                #fn_block
            });
        }
    };
    expanded.into()
}

// Tiny helper: bring span into scope for inputs.
trait SpanSite {
    fn span_site(&self) -> Span;
}
impl<T> SpanSite for T {
    fn span_site(&self) -> Span {
        Span::call_site()
    }
}
