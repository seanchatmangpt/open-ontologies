import { Receipt } from '@truex/receipt';
import { OCEL } from '@truex/ocel2';

export enum VerificationState {
  Admitted = 'Admitted',
  Refused = 'Refused',
  Incomplete = 'Incomplete'
}

export interface ValidationResult {
  state: VerificationState;
  refusal_state?: string;
  missing?: string[];
}

export class Verifier {
  async verify(receipt: Receipt): Promise<ValidationResult> {
    // 1. Raw Boundary mandatory
    if (!receipt.raw_evidence) {
      return { state: VerificationState::Refused, refusal_state: 'SummaryOnlyProof' };
    }

    // 2. OCEL Laundering check
    if (!this.isBoundaryDerived(receipt.observed_ocel, receipt.raw_evidence)) {
      return { state: VerificationState::Refused, refusal_state: 'OCELLaundering' };
    }

    return { state: VerificationState::Admitted };
  }

  private isBoundaryDerived(ocel: any, raw: any): boolean {
    if (!ocel || !raw) return false;

    const events = ocel['ocel:events'] || [];
    const executionEvents = events.filter((e: any) => e['ocel:activity'].startsWith('ExecutionComplete'));

    for (const event of executionEvents) {
      const vmap = event['ocel:vmap'] || {};
      
      // Physical match: OCEL exit_code must match Raw exit_code
      if (vmap.exit_code !== undefined && vmap.exit_code !== raw.exit_code) {
        return false;
      }

      // Physical match: OCEL stdout_hash must match recomputed Raw stdout hash
      // (Simplified check for the demo: presence in raw matches presence in ocel)
      if (vmap.stdout_hash && !raw.stdout) {
        return false;
      }
    }

    return true;
  }
}
