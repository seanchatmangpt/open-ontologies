export interface RawFilesystemMutation {
  path: string;
  hash: string;
  operation?: string;
}

export interface RawEvidence {
  exit_code?: number;
  stdout?: string;
  stderr?: string;
  filesystem_mutations?: RawFilesystemMutation[];
  [key: string]: unknown;
}

export interface OcelEvent {
  'ocel:id'?: string;
  'ocel:activity': string;
  'ocel:timestamp'?: string;
  'ocel:vmap'?: Record<string, unknown>;
}

export interface ObservedOcel {
  'ocel:events': OcelEvent[];
  [key: string]: unknown;
}

export interface ReceiptData {
  stdout_hash?: string | null;
  stderr_hash?: string | null;
  raw_evidence?: RawEvidence | null;
  raw_evidence_hash?: string | null;
  observed_ocel?: ObservedOcel | null;
  expected_ocel_hash?: string | null;
}

/** Immutable evidence carrier. Receipt construction never implies admission. */
export class Receipt {
  public readonly stdout_hash: string | null;
  public readonly stderr_hash: string | null;
  public readonly raw_evidence: RawEvidence | null;
  public readonly raw_evidence_hash: string | null;
  public readonly observed_ocel: ObservedOcel | null;
  public readonly expected_ocel_hash: string | null;

  constructor(data: ReceiptData) {
    this.stdout_hash = data.stdout_hash ?? null;
    this.stderr_hash = data.stderr_hash ?? null;
    this.raw_evidence = data.raw_evidence ?? null;
    this.raw_evidence_hash = data.raw_evidence_hash ?? null;
    this.observed_ocel = data.observed_ocel ?? null;
    this.expected_ocel_hash = data.expected_ocel_hash ?? null;
    Object.freeze(this);
  }
}
