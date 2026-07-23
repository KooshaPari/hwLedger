/**
 * Shared types for the Svelte UI.
 *
 * These mirror the Hono proxy contract (`server/src/contract.ts`); if the
 * wire shape changes on the server side, this file is the single source
 * of truth that the UI components reference.
 */

export interface ResultRow {
  id: string;
  score: number;
  facets?: Partial<Facets>;
  payload?: Record<string, unknown> | null;
}

export interface Facets {
  kinds?: string[];
  modalities?: string[];
  arch_kinds?: string[];
  attention_kinds?: Array<
    | 'mha'
    | 'gqa'
    | 'mqa'
    | 'mla'
    | 'sliding'
    | 'ssm'
    | 'hybrid'
    | 'sink'
  >;
  min_param_total?: number | null;
  max_param_total?: number | null;
  min_agentic_fit?: number | null;
  min_coding_fit?: number | null;
  license?: string | null;
  has_evals?: boolean | null;
  quants?: string[];
  provenance?: string | null;
}

export interface SearchRequest {
  text?: string;
  facets?: Partial<Facets>;
  sort?: string | null;
  limit?: number;
}

export interface SearchResponse {
  query: string;
  limit: number;
  results: ResultRow[];
}

export interface ModelDetail {
  id: string;
  found: boolean;
  score?: number | null;
  kind?: string | null;
  quants?: string[] | null;
}

export interface QuantsResponse {
  id: string;
  quants: string[];
}

export interface SimilarResponse {
  seed: string;
  limit: number;
  results: ResultRow[];
}

export type UseCaseSlug = 'agentic' | 'coding' | 'reasoning' | 'embedding';

export const USE_CASES: ReadonlyArray<{
  slug: UseCaseSlug;
  label: string;
  description: string;
}> = [
  {
    slug: 'coding',
    label: 'Coding',
    description: 'Code-completion + refactor assistants',
  },
  {
    slug: 'agentic',
    label: 'Agentic',
    description: 'Tool-using, function-calling workloads',
  },
  {
    slug: 'reasoning',
    label: 'Reasoning',
    description: 'o1 / R1 style chain-of-thought',
  },
  {
    slug: 'embedding',
    label: 'Embedding',
    description: 'Retrieval + dense vector lookup',
  },
] as const;

export interface ModelAskRequest {
  question: string;
  limit?: number;
}

export interface ModelAskContext {
  id: string;
  score: number;
  snippet: string;
}

export interface ModelAskResponse {
  question: string;
  limit: number;
  answer: string;
  context: ModelAskContext[];
}

export interface HealthResponse {
  status: 'ok';
  upstream: 'rust' | 'synthesized';
  upstream_url: string;
}

/**
 * Coarse-grained UI-friendly facet shape for the sidebar. The server
 * permits arbitrary filters; we group the most common ones and let the
 * rest travel through the {@link SearchRequest.facets} field as-is.
 */
export interface UIFacetState {
  /** Multi-select model kinds. */
  kinds: string[];
  /** Multi-select modalities (text/code/vision/…). */
  modalities: string[];
  /** Free-text lower bound on total parameter count. */
  minParams: string;
  /** Free-text upper bound. */
  maxParams: string;
  /** Restrict to models with at least one of these quants. */
  quants: string[];
  /** Exact license string match (optional). */
  license: string;
}

/** Default empty UI facet state. */
export const EMPTY_UI_FACETS: UIFacetState = {
  kinds: [],
  modalities: [],
  minParams: '',
  maxParams: '',
  quants: [],
  license: '',
};

/**
 * Convert the {@link UIFacetState} into the wire-shape `Facets` that the
 * Hono proxy / Rust server understand. Numbers are only included when
 * they're parseable — the server rejects malformed numeric ranges
 * anyway, but we'd rather not send garbage in the first place.
 */
export function facetsToFacets(state: UIFacetState): Partial<Facets> {
  const facets: Partial<Facets> = {};
  if (state.kinds.length) facets.kinds = state.kinds.slice();
  if (state.modalities.length) facets.modalities = state.modalities.slice();
  if (state.quants.length) facets.quants = state.quants.slice();

  const min = Number(state.minParams);
  if (Number.isFinite(min) && min > 0) facets.min_param_total = min;
  const max = Number(state.maxParams);
  if (Number.isFinite(max) && max > 0) facets.max_param_total = max;

  if (state.license.trim()) facets.license = state.license.trim();
  return facets;
}

/** Reverse mapping — useful when hydrating from URL query params. */
export function facetsFromFacets(f: Partial<Facets> | undefined): UIFacetState {
  if (!f) return { ...EMPTY_UI_FACETS };
  return {
    kinds: f.kinds ? [...f.kinds] : [],
    modalities: f.modalities ? [...f.modalities] : [],
    minParams:
      typeof f.min_param_total === 'number' ? String(f.min_param_total) : '',
    maxParams:
      typeof f.max_param_total === 'number' ? String(f.max_param_total) : '',
    quants: f.quants ? [...f.quants] : [],
    license: typeof f.license === 'string' ? f.license : '',
  };
}
