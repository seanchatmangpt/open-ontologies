import { sha256Digest, sha256Text } from '@truex/canonical';
import { OcelEvent, RawEvidence, Receipt } from '@truex/receipt';

export enum VerificationState {
  Admitted = 'Admitted',
  Refused = 'Refused',
  Incomplete = 'Incomplete'
}

export enum RefusalState {
  ArtifactOriginMismatch = 'ArtifactOriginMismatch',
  BoundaryProjectionFailure = 'BoundaryProjectionFailure',
  MissingBoundary = 'MissingBoundary',
  NonDerivableExecution = 'NonDerivableExecution',
  OCELLaundering = 'OCELLaundering',
  StateTransitionMismatch = 'StateTransitionMismatch',
  SummaryOnlyProof = 'SummaryOnlyProof',
  TemporalOrderingViolation = 'TemporalOrderingViolation'
}

export interface ValidationResult {
  state: VerificationState;
  refusal_state?: RefusalState;
  detail?: string;
  missing?: string[];
  evidence_hash?: string;
  ocel_hash?: string;
}

function refused(refusal_state: RefusalState, detail: string): ValidationResult {
  return { state: VerificationState.Refused, refusal_state, detail };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/**
 * Adjudicates whether an OCEL observation is physically derivable from its raw
 * execution boundary. It never performs actuation and never treats receipt
 * construction as proof.
 */
export class Verifier {
  async verify(receipt: Receipt): Promise<ValidationResult> {
    if (!receipt.raw_evidence) {
      return refused(RefusalState.SummaryOnlyProof, 'raw boundary evidence is required');
    }
    if (!receipt.raw_evidence_hash) {
      return refused(RefusalState.MissingBoundary, 'raw_evidence_hash is required');
    }
    if (!receipt.observed_ocel) {
      return refused(RefusalState.MissingBoundary, 'observed OCEL is required');
    }

    let computedEvidenceHash: string;
    try {
      computedEvidenceHash = sha256Digest(receipt.raw_evidence);
    } catch (error) {
      return refused(
        RefusalState.BoundaryProjectionFailure,
        error instanceof Error ? error.message : 'raw evidence is not canonicalizable'
      );
    }
    if (receipt.raw_evidence_hash !== computedEvidenceHash) {
      return refused(RefusalState.OCELLaundering, 'raw evidence hash does not match canonical evidence');
    }

    const derivation = this.verifyBoundaryDerivation(receipt.observed_ocel['ocel:events'], receipt.raw_evidence);
    if (derivation) {
      return derivation;
    }

    if (receipt.stdout_hash !== null) {
      if (typeof receipt.raw_evidence.stdout !== 'string' || receipt.stdout_hash !== sha256Text(receipt.raw_evidence.stdout)) {
        return refused(RefusalState.ArtifactOriginMismatch, 'stdout hash is not derived from raw stdout');
      }
    }
    if (receipt.stderr_hash !== null) {
      if (typeof receipt.raw_evidence.stderr !== 'string' || receipt.stderr_hash !== sha256Text(receipt.raw_evidence.stderr)) {
        return refused(RefusalState.ArtifactOriginMismatch, 'stderr hash is not derived from raw stderr');
      }
    }

    let ocelHash: string;
    try {
      ocelHash = sha256Digest(receipt.observed_ocel);
    } catch (error) {
      return refused(
        RefusalState.BoundaryProjectionFailure,
        error instanceof Error ? error.message : 'observed OCEL is not canonicalizable'
      );
    }
    if (receipt.expected_ocel_hash !== null && receipt.expected_ocel_hash !== ocelHash) {
      return refused(RefusalState.OCELLaundering, 'observed OCEL hash differs from the expected observation');
    }

    return {
      state: VerificationState.Admitted,
      evidence_hash: computedEvidenceHash,
      ocel_hash: ocelHash
    };
  }

  private verifyBoundaryDerivation(events: OcelEvent[], raw: RawEvidence): ValidationResult | null {
    if (!Array.isArray(events) || events.length === 0) {
      return refused(RefusalState.BoundaryProjectionFailure, 'observed OCEL must contain events');
    }

    let executionComplete = false;
    let previousTimestamp = Number.NEGATIVE_INFINITY;
    for (const [index, event] of events.entries()) {
      if (!isRecord(event) || typeof event['ocel:activity'] !== 'string' || event['ocel:activity'].length === 0) {
        return refused(RefusalState.BoundaryProjectionFailure, `event ${index} lacks an activity`);
      }
      const activity = event['ocel:activity'];
      const vmap = event['ocel:vmap'] ?? {};
      if (!isRecord(vmap)) {
        return refused(RefusalState.BoundaryProjectionFailure, `event ${index} vmap is not an object`);
      }

      if (event['ocel:timestamp'] !== undefined) {
        const timestamp = Date.parse(event['ocel:timestamp']);
        if (!Number.isFinite(timestamp) || timestamp < previousTimestamp) {
          return refused(RefusalState.TemporalOrderingViolation, `event ${index} timestamp is invalid or out of order`);
        }
        previousTimestamp = timestamp;
      }

      if (activity.startsWith('ExecutionComplete')) {
        executionComplete = true;
        if (typeof raw.exit_code !== 'number' || vmap.exit_code !== raw.exit_code) {
          return refused(RefusalState.NonDerivableExecution, `event ${index} exit_code is not derived from raw evidence`);
        }
      } else if (vmap.exit_code !== undefined && vmap.exit_code !== raw.exit_code) {
        return refused(RefusalState.StateTransitionMismatch, `event ${index} exit_code contradicts raw evidence`);
      }

      if (vmap.stdout_hash !== undefined) {
        if (typeof raw.stdout !== 'string' || vmap.stdout_hash !== sha256Text(raw.stdout)) {
          return refused(RefusalState.OCELLaundering, `event ${index} stdout_hash is not derived from raw stdout`);
        }
      }
      if (vmap.stderr_hash !== undefined) {
        if (typeof raw.stderr !== 'string' || vmap.stderr_hash !== sha256Text(raw.stderr)) {
          return refused(RefusalState.OCELLaundering, `event ${index} stderr_hash is not derived from raw stderr`);
        }
      }

      if (vmap.artifact_path !== undefined || vmap.artifact_hash !== undefined) {
        const mutations = raw.filesystem_mutations;
        if (!Array.isArray(mutations)) {
          return refused(RefusalState.ArtifactOriginMismatch, `event ${index} claims an artifact without filesystem evidence`);
        }
        const found = mutations.some(
          (mutation) =>
            isRecord(mutation) &&
            mutation.path === vmap.artifact_path &&
            mutation.hash === vmap.artifact_hash
        );
        if (!found) {
          return refused(RefusalState.ArtifactOriginMismatch, `event ${index} artifact is not present in raw filesystem mutations`);
        }
      }
    }

    if (!executionComplete) {
      return refused(RefusalState.NonDerivableExecution, 'no ExecutionComplete event derives the execution consequence');
    }
    return null;
  }
}
