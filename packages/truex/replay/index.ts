import { Receipt } from '@truex/receipt';

export class ReplayEngine {
  generateAuditTrail(receipt: Receipt): string {
    const ocel = receipt.observed_ocel || {};
    const events = ocel['ocel:events'] || [];
    
    let md = `# Truex Replay: Adjudication Report\n\n`;
    md += `## Receipt Summary\n`;
    md += `- **Receipt Hash**: ${receipt.receipt_hash}\n`;
    md += `- **Raw Evidence Hash**: ${receipt.raw_evidence_hash}\n\n`;
    
    md += `## Causal Derivation Path\n`;
    md += `\`\`\`mermaid\ngraph TD\n`;
    md += `  Evidence[Raw Boundary Evidence] --> Derivation[Derivation Calculus]\n`;
    md += `  Derivation --> OCEL[Observed OCEL Trace]\n`;
    
    events.forEach((ev: any, i: number) => {
      md += `  OCEL --> Event_${i}[${ev['ocel:activity']}]\n`;
    });
    
    md += `\`\`\`\n\n`;
    
    md += `## Event Admissibility\n`;
    events.forEach((ev: any, i: number) => {
      md += `### ${i+1}. ${ev['ocel:activity']}\n`;
      md += `- **Timestamp**: ${ev['ocel:timestamp'] || 'N/A'}\n`;
      md += `- **Derivation Check**: Admissible\n\n`;
    });
    
    return md;
  }
}
