import { Verifier, VerificationState } from '../packages/truex/verifier';
import { Receipt } from '../packages/truex/receipt';

describe('Truex Autonomic E2E Validation: NonDerivableExitCodeTest', () => {
  /**
   * Scenario: NonDerivableExitCodeTest
   * Expected: Refused (NonDerivableExecution)
   */
  it('NonDerivableExitCodeTest', async () => {
    const verifier = new Verifier();
    const inputStr = `{'raw_evidence':{'exit_code':1,'stdout':'error'},'observed_ocel':{'ocel:events':[{'ocel:activity':'ExecutionComplete_OA-1','ocel:vmap':{'exit_code':0}}]},'raw_evidence_hash':'d30e046fc9cc2eaea7e3eb777035adb3cbd0b46af953c066dcf680007ce3f4a5'}`.replace(/'/g, '"');
    const receiptData = JSON.parse(inputStr);
    const receipt = new Receipt(receiptData);

    const result = await verifier.verify(receipt);
    expect(result.state).toBe('Refused');
    
    expect(result.refusal_state).toBe('NonDerivableExecution');
    
  });
});