export interface ReceiptData {
  stdout_hash?: string;
  stderr_hash?: string;
  raw_evidence?: any;
  raw_evidence_hash?: string;
  observed_ocel?: any;
  expected_ocel_hash?: string;
}

export class Receipt {
  public stdout_hash: string | null;
  public raw_evidence: any | null;
  public raw_evidence_hash: string | null;
  public observed_ocel: any | null;

  constructor(data: ReceiptData) {
    this.stdout_hash = data.stdout_hash || null;
    this.raw_evidence = data.raw_evidence || null;
    this.raw_evidence_hash = data.raw_evidence_hash || null;
    this.observed_ocel = data.observed_ocel || null;
  }
}
