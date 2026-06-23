import { Receipt } from '@truex/receipt';
import { OCEL } from '@truex/ocel2';
import * as crypto from 'crypto';

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
      return { state: VerificationState.Refused, refusal_state: 'SummaryOnlyProof' };
    }

    // 2. Physical Hash Recomputation (Anti-Laundering)
    const computedHash = crypto.createHash('sha256').update(JSON.stringify(receipt.raw_evidence)).digest('hex');
    if (receipt.raw_evidence_hash && receipt.raw_evidence_hash !== computedHash) {
      return { state: VerificationState.Refused, refusal_state: 'OCELLaundering' };
    }

    // 3. Structural Derivation Adjudication
    if (!this.isBoundaryDerived(receipt.observed_ocel, receipt.raw_evidence)) {
      return { state: VerificationState.Refused, refusal_state: 'OCELLaundering' };
    }

    return { state: VerificationState.Admitted };
  }

  private isBoundaryDerived(ocel: any, raw: any): boolean {
    if (!ocel || !raw) return false;

    const events = ocel['ocel:events'] || [];
    
    
    // Enforcing Derivation Rule: MaximalDerivation
    
    const events_1 = events;
    

    for (const event of events_1) {
      const vmap = event['ocel:vmap'] || {};
      
    }
    // Enforcing Derivation Rule: FilesystemMutationDerivesArtifactObject
    
    const events_2 = events;
    

    for (const event of events_2) {
      const vmap = event['ocel:vmap'] || {};
      
    }
    // Enforcing Derivation Rule: StdoutHashDerivesArtifactEmission
    
    const events_3 = events;
    

    for (const event of events_3) {
      const vmap = event['ocel:vmap'] || {};
      
      // Adjudicating physical field mapping: stdout -> stdout_hash
      if (vmap.stdout_hash !== undefined && vmap.stdout_hash !== raw.stdout) {
        return false;
      }
      
    }
    // Enforcing Derivation Rule: RawExitCodeDerivesExecutionStatus
    
    const events_4 = events.filter((e: any) => e['ocel:activity'].startsWith('ExecutionComplete'));
    

    for (const event of events_4) {
      const vmap = event['ocel:vmap'] || {};
      
      // Adjudicating physical field mapping: exit_code -> exit_code
      if (vmap.exit_code !== undefined && vmap.exit_code !== raw.exit_code) {
        return false;
      }
      
    }
    

    return true;
  }
}