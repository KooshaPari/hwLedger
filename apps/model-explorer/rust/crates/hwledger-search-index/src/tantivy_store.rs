//! Tantivy-backed BM25 store.
//!
//! One [`TantivyStore`] wraps a single Tantivy index directory. The schema is
//! defined inline (see [`build_schema`]) and is intentionally narrow: just
//! enough fields to do reasonable free-text recall over a model card.
//!
//! Public API:
//!
//! - [`TantivyStore::open`] — create or open the index at a directory.
//! - [`TantivyStore::upsert`] — replace the doc with the given `id`.
//! - [`TantivyStore::search`] — BM25 over all indexed fields with per-field
//!   boosts (name^3, org^2, kind^2, family^2, arch^1, quants^1, card_snippet^1).
//! - [`TantivyStore::commit`] — flush the writer.
//!
//! [`IndexHit`] is the crate-local result row; the higher-level
//! [`crate::query::run_hybrid`] converts these into the
//! `hwledger_search_core::FusedResult` shape.

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, STORED, STRING,
};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};

use crate::error::IndexError;

/// One BM25 hit, in the shape callers (CLI, MCP) expect.
///
/// `id` is the primary key the caller used during [`TantivyStore::upsert`].
/// `score` is the raw Tantivy BM25 score (higher = more relevant).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexHit {
    /// Primary-key id (see [`crate::ingest::IndexedModel::id`]).
    pub id: String,
    /// Tantivy BM25 score, monotonically higher for more-relevant hits.
    pub score: f32,
}

impl IndexHit {
    /// Builder helper.
    pub fn new<I: Into<String>>(id: I, score: f32) -> Self {
        Self {
            id: id.into(),
            score,
        }
    }
}

/// Holds the resolved schema field handles, plus the running Tantivy objects.
pub struct TantivyStore {
    /// The schema we wrote to disk; held so we can resolve `Field` ids cheaply
    /// on every upsert/search call.
    schema: Schema,

    /// The underlying Tantivy index. Cheap to clone (`Arc` internally).
    idx: Index,

    /// Lazily-constructed writer; we hold an `Option` because the writer is
    /// `!Send` on drop in some edge cases and we want to be able to
    /// re-create it after a `commit` / explicit drop if the caller ever
    /// exhausts it.
    writer: Mutex<Option<IndexWriter>>,

    /// Cached reader used by `search`. The default reload policy is
    /// `OnCommitWithDelay`; we additionally call `reload()` after each commit
    /// for deterministic test behaviour.
    reader: IndexReader,

    /// Resolved field handles.
    fields: SchemaFields,

    /// Sidecar: per-id `kind` (untokenized string). We keep this in-process
    /// because the v1 Tantivy schema treats `kind` as a tokenized text
    /// field — fast and good for BM25, but expensive to round-trip on a
    /// post-filter lookup. The sidecar is updated by every [`upsert`].
    kinds: Arc<Mutex<std::collections::HashMap<String, String>>>,

    /// Sidecar: per-id `quants` list (split on whitespace).
    quants: Arc<Mutex<std::collections::HashMap<String, Vec<String>>>>,
}

#[derive(Debug, Clone)]
struct SchemaFields {
    id: Field,
    name: Field,
    org: Field,
    kind: Field,
    family: Field,
    arch: Field,
    quants: Field,
    card_snippet: Field,
}

/// Build the (immutable) schema for the store.
///
/// The schema is the same whether we're creating or opening; tantivy will
/// validate it against `meta.json` on open.
fn build_schema() -> (Schema, SchemaFields) {
    let mut b = Schema::builder();

    // `id` is the primary key. STRING = untokenized + indexed; STORED so we
    // can pull it back out of search hits without re-walking the doc.
    let id = b.add_text_field("id", STRING | STORED);

    // The remaining fields are full-text searchable. Default tokenization +
    // positions. We deliberately do *not* set `STORED` on these — we never
    // need to retrieve them; only the `id` is read back.
    let text_indexing = TextFieldIndexing::default()
        .set_tokenizer("default")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);

    let name = b.add_text_field("name", TextOptions::default().set_indexing_options(text_indexing.clone()));
    let org = b.add_text_field("org", TextOptions::default().set_indexing_options(text_indexing.clone()));
    let kind = b.add_text_field("kind", TextOptions::default().set_indexing_options(text_indexing.clone()));
    let family = b.add_text_field("family", TextOptions::default().set_indexing_options(text_indexing.clone()));
    let arch = b.add_text_field("arch", TextOptions::default().set_indexing_options(text_indexing.clone()));
    let quants = b.add_text_field("quants", TextOptions::default().set_indexing_options(text_indexing.clone()));
    let card_snippet = b.add_text_field(
        "card_snippet",
        TextOptions::default().set_indexing_options(text_indexing),
    );

    let schema = b.build();
    let fields = SchemaFields {
        id,
        name,
        org,
        kind,
        family,
        arch,
        quants,
        card_snippet,
    };
    (schema, fields)
}

impl TantivyStore {
    /// Open (or create) the index at `path`. The path must be a directory;
    /// tantivy will create the directory if it doesn't already exist.
    ///
    /// If the directory exists but doesn't yet contain a Tantivy `meta.json`
    /// (e.g. a freshly-created `tempfile::tempdir()`), we treat it as a
    /// *new* index and call `Index::create_in_dir` — this is friendlier to
    /// test harnesses that pre-create the directory.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, IndexError> {
        let path = path.as_ref();
        let (schema, fields) = build_schema();

        let meta_exists = path.join("meta.json").exists();
        let idx = if meta_exists {
            // Open existing. tantivy reads meta.json and rejects mismatched
            // schemas; we propagate the error.
            Index::open_in_dir(path)?
        } else {
            // Create new. Ensure the directory exists; tantivy's
            // `create_in_dir` will populate `meta.json` + segments/.
            std::fs::create_dir_all(path).map_err(IndexError::from)?;
            Index::create_in_dir(path, schema.clone())?
        };

        let writer = idx.writer(50_000_000)?;
        let reader = idx
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(Self {
            schema,
            idx,
            writer: Mutex::new(Some(writer)),
            reader,
            fields,
            kinds: Arc::new(Mutex::new(std::collections::HashMap::new())),
            quants: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }

    /// Return a reference to the underlying schema (mainly useful in tests).
    #[must_use]
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Replace (or insert) the document whose `id` equals `id`.
    ///
    /// Implementation note: tantivy's "update" model is `delete_term` +
    /// `add_document`. The delete only takes effect after `commit()`; we
    /// commit lazily — see [`TantivyStore::commit`].
    pub fn upsert(
        &self,
        id: &str,
        name: &str,
        org: &str,
        kind: &str,
        family: &str,
        arch: &str,
        quants: &str,
        card_snippet: &str,
    ) -> Result<(), IndexError> {
        if id.is_empty() {
            return Err(IndexError::InvalidArgs("id is empty".into()));
        }

        let mut guard = self
            .writer
            .lock()
            .map_err(|e| IndexError::Tantivy(format!("writer mutex poisoned: {e}")))?;
        let writer = guard
            .as_mut()
            .ok_or_else(|| IndexError::Tantivy("writer is missing (was committed away?)".into()))?;

        // Delete-by-id before add; the delete will be visible at the next
        // commit, at which point we'll have only the new doc.
        let id_term = Term::from_field_text(self.fields.id, id);
        writer.delete_term(id_term);

        let mut doc = TantivyDocument::default();
        doc.add_text(self.fields.id, id);
        doc.add_text(self.fields.name, name);
        doc.add_text(self.fields.org, org);
        doc.add_text(self.fields.kind, kind);
        doc.add_text(self.fields.family, family);
        doc.add_text(self.fields.arch, arch);
        doc.add_text(self.fields.quants, quants);
        doc.add_text(self.fields.card_snippet, card_snippet);

        writer.add_document(doc)?;

        // Sidecar refresh: replace this id's kind + quants so a follow-up
        // post-filter lookup is O(1).
        if let Ok(mut kinds_map) = self.kinds.lock() {
            kinds_map.insert(id.to_string(), kind.to_string());
        }
        if let Ok(mut quants_map) = self.quants.lock() {
            let split: Vec<String> = quants
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            quants_map.insert(id.to_string(), split);
        }
        Ok(())
    }

    /// Look up the (untokenized) `kind` string we stored at [`upsert`] time.
    #[must_use]
    pub fn kind_for_id(&self, id: &str) -> Option<String> {
        self.kinds.lock().ok().and_then(|m| m.get(id).cloned())
    }

    /// Look up the list of quantization tags we stored at [`upsert`] time.
    #[must_use]
    pub fn quants_for_id(&self, id: &str) -> Option<Vec<String>> {
        self.quants.lock().ok().and_then(|m| m.get(id).cloned())
    }

    /// Flush the writer to disk and force the reader to pick up the new
    /// segment.
    pub fn commit(&self) -> Result<(), IndexError> {
        let mut guard = self
            .writer
            .lock()
            .map_err(|e| IndexError::Tantivy(format!("writer mutex poisoned: {e}")))?;
        let writer = guard
            .as_mut()
            .ok_or_else(|| IndexError::Tantivy("writer is missing".into()))?;
        writer.commit()?;
        // Reload so a follow-up `search()` in the same process observes the
        // commit without waiting for the policy delay.
        self.reader.reload()?;
        Ok(())
    }

    /// Run a BM25 query against the indexed fields and return up to `k`
    /// hits, sorted by score descending.
    ///
    /// The query is parsed with Tantivy's [`QueryParser`] over the
    /// default-fields list `[name^3, org^2, kind^2, family^2, arch^1,
    /// quants^1, card_snippet^1]`; per-field boosts multiply the user's
    /// own `^N` syntax if they use it.
    pub fn search(&self, q: &str, k: usize) -> Result<Vec<IndexHit>, IndexError> {
        if k == 0 {
            return Ok(Vec::new());
        }

        let mut parser = QueryParser::for_index(
            &self.idx,
            vec![
                self.fields.name,
                self.fields.org,
                self.fields.kind,
                self.fields.family,
                self.fields.arch,
                self.fields.quants,
                self.fields.card_snippet,
            ],
        );
        // Per-field BM25 boost multipliers. The QueryParser multiplies these
        // with any user-supplied ^N.
        parser.set_field_boost(self.fields.name, 3.0);
        parser.set_field_boost(self.fields.org, 2.0);
        parser.set_field_boost(self.fields.kind, 2.0);
        parser.set_field_boost(self.fields.family, 2.0);
        parser.set_field_boost(self.fields.arch, 1.0);
        parser.set_field_boost(self.fields.quants, 1.0);
        parser.set_field_boost(self.fields.card_snippet, 1.0);

        // `parse_query_lenient` returns a "match nothing" subquery for
        // unparseable input instead of erroring out — we still produce
        // *some* BM25 hits if any term is valid.
        let (query, _parse_errors) = parser.parse_query_lenient(q);

        let searcher = self.reader.searcher();
        let top: Vec<(f32, tantivy::DocAddress)> = searcher.search(&*query, &TopDocs::with_limit(k))?;

        if top.is_empty() {
            return Ok(Vec::new());
        }

        let mut hits = Vec::with_capacity(top.len());
        for (score, addr) in top {
            let stored: TantivyDocument = searcher.doc(addr)?;
            // `id` is STRING | STORED; pull the first value.
            let id = stored
                .get_first(self.fields.id)
                .and_then(|v| match v {
                    tantivy::schema::OwnedValue::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            hits.push(IndexHit::new(id, score));
        }
        Ok(hits)
    }
}